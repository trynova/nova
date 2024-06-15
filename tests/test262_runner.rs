use clap::Parser as ClapParser;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::OsStr,
    fs::{read_dir, File},
    io::{ErrorKind, Read},
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum TestExpectation {
    Pass,
    Fail,
    Crash,
    Timeout,
}

/// These directory names are filtered out by default.
const SKIP_DIRS: &[&str] = &[
    "annexB", "intl402", "staging",
    //
    //"built-ins",
    //"language",
];

fn is_test_file(file_name: &str) -> bool {
    file_name.ends_with(".js") && !file_name.contains("_FIXTURE")
}

#[derive(Debug)]
struct Test262Runner {
    runner_base_path: PathBuf,
    tests_base: PathBuf,
    nova_harness_path: PathBuf,
    nova_cli_path: PathBuf,
    expectations: HashMap<PathBuf, TestExpectation>,
    treat_crashes_as_failures: bool,

    num_tests_run: usize,
    unexpected_results: HashMap<PathBuf, TestExpectation>,
}

impl Test262Runner {
    const TEST_TIMEOUT: Duration = Duration::from_secs(10);

    /// This error code denotes a parse error or an uncaught exception, all
    /// others are treated as crashes.
    const FAILURE_ERROR_CODE: i32 = 1;

    /// If `filters` is empty, it means run every test. Otherwise, only tests
    /// which have one of the entries of `filters` as its prefix should run.
    pub fn run(&mut self, filters: &[PathBuf]) {
        self.walk_dir(&self.tests_base.clone(), filters);
    }

    /// If `filters` is empty, every test in this directory and its descendants
    /// should be run. Otherwise, only tests which have one of the entries of
    /// `filters` as its prefix (considering their relative path to the `path`
    /// directory) will be run.
    fn walk_dir(&mut self, path: &PathBuf, filters: &[PathBuf]) {
        for entry in read_dir(path).unwrap() {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_dir() {
                if let Some(child_filters) = Self::filter_dir(filters, &entry.file_name()) {
                    self.walk_dir(&entry.path(), &child_filters);
                }
            }

            if entry.file_type().unwrap().is_file()
                && Self::filter_file(filters, &entry.file_name())
            {
                self.run_test(&entry.path());
            }
        }
    }

    fn filter_dir(filters: &[PathBuf], os_file_name: &OsStr) -> Option<Box<[PathBuf]>> {
        if SKIP_DIRS.contains(&os_file_name.to_str().unwrap()) {
            return None;
        }
        if filters.is_empty() {
            return Some(vec![].into_boxed_slice());
        }

        let mut child_filters = Vec::with_capacity(filters.len());
        for filter in filters {
            if filter == OsStr::new("") || filter == os_file_name {
                return Some(vec![].into_boxed_slice());
            }
            if let Ok(child_filter) = filter.strip_prefix(os_file_name) {
                child_filters.push(child_filter.to_path_buf());
            }
        }

        if child_filters.is_empty() {
            None
        } else {
            Some(child_filters.into_boxed_slice())
        }
    }

    fn filter_file(filters: &[PathBuf], os_file_name: &OsStr) -> bool {
        if !is_test_file(os_file_name.to_str().unwrap()) {
            return false;
        }
        if filters.is_empty() {
            return true;
        }
        filters.iter().any(|filter| filter == os_file_name)
    }

    fn run_test(&mut self, path: &PathBuf) {
        //println!("Running test {:?}", path);
        let metadata = test_metadata::parse(path);

        if metadata.flags.is_async || metadata.flags.module {
            // We don't yet support async or modules, skip any tests for them.
            return;
        }

        self.num_tests_run += 1;

        let mut command = Command::new(&self.nova_cli_path);
        command.arg("eval");

        command.arg(&self.nova_harness_path);
        if metadata.flags.raw {
            assert!(metadata.includes.is_empty());
        } else {
            let mut harness = self.runner_base_path.clone();
            harness.push("test262/harness");
            let includes_iter = ["assert.js", "sta.js"]
                .iter()
                .map(PathBuf::from)
                .chain(metadata.includes);
            for include_relpath in includes_iter {
                let mut include = harness.clone();
                include.push(include_relpath);
                command.arg(include);
            }
        }
        command.arg(path);

        //println!("{:?}", command);

        let test_result = Self::handle_test_output(command, &metadata.negative);

        let relpath = path.strip_prefix(&self.tests_base).unwrap();
        let expectation = self
            .expectations
            .get(relpath)
            .copied()
            .unwrap_or(TestExpectation::Pass);

        if test_result != expectation {
            if self.treat_crashes_as_failures
                && matches!(test_result, TestExpectation::Fail | TestExpectation::Crash)
                && matches!(expectation, TestExpectation::Fail | TestExpectation::Crash)
            {
                return;
            }
            self.unexpected_results
                .insert(relpath.to_path_buf(), test_result);
        }
    }

    fn handle_test_output(
        mut command: Command,
        negative: &Option<test_metadata::NegativeExpectation>,
    ) -> TestExpectation {
        command.stdout(Stdio::null()).stderr(Stdio::piped());

        let mut child = command.spawn().unwrap();

        let Some(status) = child.wait_timeout(Self::TEST_TIMEOUT).unwrap() else {
            child.kill().unwrap();
            child.wait().unwrap();
            return TestExpectation::Timeout;
        };

        if !status.success() && status.code() != Some(Self::FAILURE_ERROR_CODE) {
            return TestExpectation::Crash;
        }

        let pass = if status.success() {
            negative.is_none()
        } else if let Some(negative) = negative {
            let expected_stderr_prefix: Cow<str> = match negative.phase {
                test_metadata::TestFailurePhase::Parse => "Parse errors:".into(),
                test_metadata::TestFailurePhase::Runtime => {
                    format!("Uncaught exception: {}", negative.error_type).into()
                }
                test_metadata::TestFailurePhase::Resolution => {
                    // Module tests should have bailed out earlier, so we
                    // shouldn't ever reach this point.
                    unreachable!()
                }
            };

            let mut buffer = vec![0u8; expected_stderr_prefix.len()];
            match child.stderr.unwrap().read_exact(&mut buffer) {
                Ok(_) => buffer == expected_stderr_prefix.as_bytes(),
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        false
                    } else {
                        panic!("{:?}", e);
                    }
                }
            }
        } else {
            false
        };

        if pass {
            TestExpectation::Pass
        } else {
            TestExpectation::Fail
        }
    }
}

mod test_metadata {
    use std::path::PathBuf;

    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum TestFailurePhase {
        Parse,
        Resolution,
        Runtime,
    }

    #[derive(Deserialize, Debug)]
    pub struct NegativeExpectation {
        pub phase: TestFailurePhase,
        #[serde(rename = "type")]
        pub error_type: String,
    }

    #[derive(Debug, Default)]
    pub struct Flags {
        pub strict: Option<bool>,
        pub module: bool,
        pub raw: bool,
        pub is_async: bool,
        pub generated: bool,
        pub can_block: Option<bool>,
        pub non_deterministic: bool,
    }
    impl<'de> Deserialize<'de> for Flags {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Flags;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a sequence")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    use serde::de::Error;
                    let mut flags = Flags::default();
                    while let Some(flag) = seq.next_element()? {
                        match flag {
                            "onlyStrict" => {
                                assert!(flags.strict.is_none());
                                flags.strict = Some(true);
                            }
                            "noStrict" => {
                                assert!(flags.strict.is_none());
                                flags.strict = Some(false);
                            }
                            "module" => {
                                assert!(!flags.module);
                                assert!(flags.strict.is_none());
                                flags.module = true;
                            }
                            "raw" => {
                                assert!(!flags.raw);
                                assert!(flags.strict.is_none());
                                flags.raw = true;
                            }
                            "async" => {
                                assert!(!flags.is_async);
                                flags.is_async = true;
                            }
                            "generated" => {
                                assert!(!flags.generated);
                                flags.generated = true;
                            }
                            "CanBlockIsFalse" => {
                                assert!(flags.can_block.is_none());
                                flags.can_block = Some(false);
                            }
                            "CanBlockIsTrue" => {
                                assert!(flags.can_block.is_none());
                                flags.can_block = Some(true);
                            }
                            "non-deterministic" => {
                                assert!(!flags.non_deterministic);
                                flags.non_deterministic = true;
                            }
                            _ => return Err(A::Error::custom("Unexpected test262 flag")),
                        };
                    }
                    Ok(flags)
                }
            }

            deserializer.deserialize_seq(Visitor)
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct TestMetadata {
        pub negative: Option<NegativeExpectation>,
        #[serde(default)]
        pub includes: Vec<PathBuf>,
        #[serde(default)]
        pub flags: Flags,
    }

    struct BytesMatcher {
        needle: &'static [u8],
        /// The next index into the needle that should match.
        idx: usize,
    }
    impl BytesMatcher {
        fn new(needle: &'static [u8]) -> Self {
            BytesMatcher { needle, idx: 0 }
        }
        fn run(&mut self, haystack: &[u8]) -> Option<usize> {
            for (i, b) in haystack.iter().enumerate() {
                if *b == self.needle[self.idx] {
                    self.idx += 1;
                    if self.idx == self.needle.len() {
                        return Some(i + 1);
                    }
                    assert!(self.idx < self.needle.len());
                } else if self.idx != 0 {
                    // Backtrack. The last `self.idx` bytes we saw match the
                    // start of the needle, so we can backtrack based only on
                    // the needle, without having had to store the previous
                    // haystack we checked.
                    let prev_idx = self.idx;
                    self.idx = 0;

                    let last_bytes = &self.needle[..prev_idx];
                    for new_idx in (0..prev_idx).rev() {
                        if last_bytes.ends_with(&self.needle[..new_idx])
                            && *b == self.needle[new_idx]
                        {
                            self.idx = new_idx + 1;
                            break;
                        }
                    }
                }
            }
            None
        }
    }

    pub fn parse(path: &PathBuf) -> TestMetadata {
        use std::fs::File;
        use std::io::{ErrorKind, Read};

        const YAML_START: &[u8] = b"/*---";
        const YAML_END: &[u8] = b"---*/";

        let mut reader = File::open(path).unwrap();

        let mut bytes = vec![];
        let mut buffer = [0u8; 1024];

        // Consume until the start of the YAML declaration
        let mut matcher = BytesMatcher::new(YAML_START);
        loop {
            let read_bytes = match reader.read(&mut buffer) {
                Ok(read_bytes) => read_bytes,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => panic!("{:?}", error),
            };

            assert_ne!(
                read_bytes, 0,
                "Expected YAML declaration start before the end of the file"
            );

            let read_slice = &buffer[..read_bytes];
            if let Some(idx) = matcher.run(read_slice) {
                bytes.extend_from_slice(&read_slice[idx..]);
                break;
            }
        }

        // Check if we've already found the end of the YAML declaration.
        matcher = BytesMatcher::new(YAML_END);
        if let Some(idx) = matcher.run(&bytes) {
            assert!(idx >= YAML_END.len());
            bytes.truncate(idx - YAML_END.len());
        } else {
            // Consume until the end of the YAML declaration.
            loop {
                let read_bytes = match reader.read(&mut buffer) {
                    Ok(read_bytes) => read_bytes,
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => panic!("{:?}", error),
                };

                assert_ne!(
                    read_bytes, 0,
                    "Expected YAML declaration end before the end of the file"
                );

                let read_slice = &buffer[..read_bytes];
                if let Some(idx) = matcher.run(read_slice) {
                    if idx > YAML_END.len() {
                        bytes.extend_from_slice(&read_slice[..idx - YAML_END.len()]);
                    } else {
                        bytes.truncate(bytes.len() + idx - YAML_END.len());
                    }
                    break;
                } else {
                    bytes.extend_from_slice(read_slice);
                }
            }
        }

        serde_yml::from_slice(&bytes).unwrap()
    }
}

#[derive(Debug, ClapParser)]
#[command(name = "test262")]
#[command(about = "A test262 runner for Nova.", long_about = None)]
struct Cli {
    #[arg(short, long)]
    update_expectations: bool,
    #[arg(long = "dont-treat-crashes-as-failures", action = clap::ArgAction::SetFalse)]
    treat_crashes_as_failures: bool,
    filters: Vec<PathBuf>,
}

fn main() {
    let mut cli = Cli::parse();

    // We're expecting this binary to always be run in the same machine at
    // the same time as the repo checkout exists.
    let runner_base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_base = runner_base_path.join("test262/test");
    let nova_harness_path = runner_base_path.join("nova-harness.js");

    let nova_cli_path = {
        // current_exe should be target/release/test262(.exe)
        // The nova_cli binary is target/release/nova_cli(.exe)
        let mut path = std::env::current_exe().unwrap();
        assert!(path.pop());
        path.push("nova_cli");
        if cfg!(windows) {
            path.set_extension(".exe");
        }
        assert!(path.is_file());
        path
    };

    let expectation_path = runner_base_path.join("expectations.json");
    let expectations = {
        if cli.update_expectations && !expectation_path.is_file() {
            Default::default()
        } else {
            let file = File::open(&expectation_path).unwrap();
            if cli.update_expectations && file.metadata().unwrap().len() == 0 {
                Default::default()
            } else {
                let read_result = serde_json::from_reader(&file);
                if cli.update_expectations
                    && read_result.is_err()
                    && !read_result.as_ref().unwrap_err().is_io()
                {
                    // Failed to parse JSON, but it's okay since we're updating
                    // the expectations file anyway.
                    Default::default()
                } else {
                    read_result.unwrap()
                }
            }
        }
    };

    // Preprocess filters
    let mut filters_are_valid = true;
    for filter in cli.filters.iter_mut() {
        let absolute = tests_base.join(&*filter);
        assert!(absolute.is_absolute());
        let Ok(canonical) = absolute.canonicalize() else {
            filters_are_valid = false;
            break;
        };
        assert!(canonical.is_absolute());
        let Ok(relative) = canonical.strip_prefix(&tests_base) else {
            filters_are_valid = false;
            break;
        };
        if *filter != relative {
            *filter = relative.to_path_buf();
        }
    }

    let mut runner = Test262Runner {
        runner_base_path,
        tests_base,
        nova_harness_path,
        nova_cli_path,
        treat_crashes_as_failures: cli.treat_crashes_as_failures,
        expectations,
        num_tests_run: 0,
        unexpected_results: Default::default(),
    };
    if filters_are_valid {
        runner.run(&cli.filters);
    }

    //println!();
    //println!();

    if runner.num_tests_run == 0 {
        println!("No tests found. Check your filters.");
        std::process::exit(1);
    }

    if runner.unexpected_results.is_empty() {
        println!("No unexpected test results");
    } else if !cli.update_expectations {
        println!(
            "Found {} unexpected test results:",
            runner.unexpected_results.len()
        );
        for (path, result) in &runner.unexpected_results {
            let expectation = runner
                .expectations
                .get(path)
                .copied()
                .unwrap_or(TestExpectation::Pass);
            println!(
                "\t{:?} -- Expected {:?}, got {:?}",
                path, expectation, result
            );
        }

        std::process::exit(1);
    } else {
        println!(
            "Updating the expectations file with {} unexpected test results.",
            runner.unexpected_results.len()
        );

        let mut file = File::create(expectation_path).unwrap();
        runner.expectations.extend(runner.unexpected_results);

        // We convert to a JSON value first because that way the paths are
        // ordered alphabetically.
        // See https://stackoverflow.com/questions/67789198.
        let json = serde_json::to_value(runner.expectations).unwrap();
        serde_json::to_writer_pretty(&mut file, &json).unwrap();
    }
}
