use clap::{Args, Parser as ClapParser, Subcommand};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    ffi::OsStr,
    fs::{read_dir, File},
    io::{ErrorKind, Read, Write},
    num::NonZeroUsize,
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

/// Directory names to always filter out.
///
/// - `annexB`: Annex B of the ES specification defines legacy syntax, methods
///   and other behaviors which are only needed for web compatibility. At this
///   point we don't plan on implementing them.
/// - `intl402`: Tests for ECMA-402, which defines the `Intl`
///   internationalization API. We currently have no plans to implement them.
const SKIP_DIRS: &[&str] = &["annexB", "intl402"];

fn is_test_file(file_name: &str) -> bool {
    // File names containing the string "_FIXTURE" are JS modules which get
    // imported by module tests. They should not be run as standalone tests.
    // See https://github.com/tc39/test262/blob/main/INTERPRETING.md#modules
    file_name.ends_with(".js") && !file_name.contains("_FIXTURE")
}

#[derive(Default)]
struct TestFilters {
    allowlist: Vec<PathBuf>,
    denylist: Vec<PathBuf>,
}

impl TestFilters {
    /// Checks if a directory should be filtered out, and if not, returns the
    /// [`TestFilters`] object for its children.
    fn filter_dir(&self, os_folder_name: &OsStr) -> Option<TestFilters> {
        // No filters means that all directories (and valid test files within
        // them) should be visited.
        if self.allowlist.is_empty() && self.denylist.is_empty() {
            return Some(TestFilters::default());
        }

        let mut child_denylist = Vec::with_capacity(self.denylist.len());
        for filter in self.denylist.iter() {
            // If a denylist filter exactly matches this folder, it should be
            // filtered out. We also do this is we happen to have an empty
            // string as a filter (e.g. because the filter path ended with a
            // slash), since that means our parent directory should have been
            // filtered out.
            if filter == OsStr::new("") || filter == os_folder_name {
                return None;
            }
            if let Ok(child_filter) = filter.strip_prefix(os_folder_name) {
                child_denylist.push(child_filter.to_path_buf());
            }
        }

        // An empty allowlist means that everything that isn't explicitly denied
        // is allowed.
        if self.allowlist.is_empty() {
            return Some(TestFilters {
                allowlist: vec![],
                denylist: child_denylist,
            });
        }

        let mut child_allowlist = Vec::with_capacity(self.allowlist.len());
        for filter in self.allowlist.iter() {
            // If we find a filter that exactly matches this folder, then all of
            // its descendants should be visited, unless they're explicitly in
            // the denylist. This is also the case if the filter is an empty
            // string, see above on the denylist for why.
            if filter == OsStr::new("") || filter == os_folder_name {
                return Some(TestFilters {
                    allowlist: vec![],
                    denylist: child_denylist,
                });
            }
            if let Ok(child_filter) = filter.strip_prefix(os_folder_name) {
                child_allowlist.push(child_filter.to_path_buf());
            }
        }

        if child_allowlist.is_empty() {
            // We used to have allowlist filters, but none of them apply to this
            // folder or to any of its descendants, so we're denied.
            None
        } else {
            Some(TestFilters {
                allowlist: child_allowlist,
                denylist: child_denylist,
            })
        }
    }

    fn filter_file(&self, os_file_name: &OsStr) -> bool {
        if let Some(str_file_name) = os_file_name.to_str() {
            if !is_test_file(str_file_name) {
                return false;
            }
        }
        if self.denylist.iter().any(|path| path == os_file_name) {
            return false;
        }
        if self.allowlist.is_empty() {
            return true;
        }
        self.allowlist.iter().any(|path| path == os_file_name)
    }
}

#[derive(Debug)]
struct BaseTest262Runner {
    runner_base_path: PathBuf,
    tests_base: PathBuf,
    nova_harness_path: PathBuf,
    nova_cli_path: PathBuf,
    print_progress: bool,
    in_test_eval: bool,
}

impl BaseTest262Runner {
    const TEST_TIMEOUT: Duration = Duration::from_secs(10);

    /// This error code denotes a parse error or an uncaught exception, all
    /// others are treated as crashes.
    const FAILURE_ERROR_CODE: i32 = 1;

    fn run_test(&self, path: &PathBuf) -> Option<TestExpectation> {
        let metadata = test_metadata::parse(path);

        if metadata.flags.is_async || metadata.flags.module {
            // We don't yet support async or modules, skip any tests for them.
            return None;
        }

        if self.print_progress {
            let relpath = path.strip_prefix(&self.tests_base).unwrap();
            let mut message = format!("Running {}", relpath.to_string_lossy());
            if message.len() > 80 {
                message.truncate(80 - 3);
                message.push_str("...");
            }
            // These escape codes make this line overwrite the previous line.
            print!("{}\x1B[0K\r", message);
        }

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
                .chain(metadata.includes.iter().cloned());
            for include_relpath in includes_iter {
                let mut include = harness.clone();
                include.push(include_relpath);
                command.arg(include);
            }
        }
        command.arg(path);

        if self.in_test_eval {
            println!("Running: {:?}", command);
            println!();
        }

        Some(self.run_command_and_parse_output(command, &metadata.negative))
    }

    fn run_command_and_parse_output(
        &self,
        mut command: Command,
        negative: &Option<test_metadata::NegativeExpectation>,
    ) -> TestExpectation {
        if self.in_test_eval {
            command.stdout(Stdio::inherit());
        } else {
            command.stdout(Stdio::null());
        }
        command.stderr(Stdio::piped());

        let mut child = command.spawn().unwrap();

        let Some(status) = child.wait_timeout(Self::TEST_TIMEOUT).unwrap() else {
            child.kill().unwrap();
            child.wait().unwrap();
            if self.in_test_eval {
                std::io::copy(&mut child.stderr.unwrap(), &mut std::io::stderr()).unwrap();
                std::io::stderr().flush().unwrap();
            }
            return TestExpectation::Timeout;
        };

        if !status.success() && status.code() != Some(Self::FAILURE_ERROR_CODE) {
            if self.in_test_eval {
                std::io::copy(&mut child.stderr.unwrap(), &mut std::io::stderr()).unwrap();
                std::io::stderr().flush().unwrap();
            }
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
            match child.stderr.as_mut().unwrap().read_exact(&mut buffer) {
                Ok(_) => {
                    if self.in_test_eval {
                        std::io::stderr().write_all(&buffer).unwrap();
                    }
                    buffer == expected_stderr_prefix.as_bytes()
                }
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

        if self.in_test_eval {
            std::io::copy(&mut child.stderr.unwrap(), &mut std::io::stderr()).unwrap();
            std::io::stderr().flush().unwrap();
        }

        if pass {
            TestExpectation::Pass
        } else {
            TestExpectation::Fail
        }
    }
}

#[derive(Debug)]
struct Test262Runner {
    inner: BaseTest262Runner,
    expectations: HashMap<PathBuf, TestExpectation>,
    treat_crashes_as_failures: bool,
}

#[derive(Debug, Default)]
struct Test262RunnerState {
    num_tests_run: usize,
    unexpected_results: HashMap<PathBuf, TestExpectation>,
}

thread_local! {
    static RUNNER_STATE: RefCell<Test262RunnerState> = Default::default();
}

impl Test262Runner {
    /// If `filters` is empty, it means run every test. Otherwise, only tests
    /// which have one of the entries of `filters` as its prefix should run.
    pub fn run(
        &self,
        filters: &TestFilters,
        num_threads: Option<NonZeroUsize>,
    ) -> Test262RunnerState {
        let thread_pool = {
            let mut builder = rayon::ThreadPoolBuilder::new();
            if let Some(num_threads) = num_threads {
                builder = builder.num_threads(num_threads.get());
            };
            builder.use_current_thread().build().unwrap()
        };

        thread_pool.install(|| {
            self.walk_dir(&self.inner.tests_base.clone(), filters);
        });

        // Clear the previous line.
        if self.inner.print_progress {
            print!("\x1B[2K\r");
        }

        // Get the runner state for each thread, and merge them together.
        thread_pool
            .broadcast(|_| RUNNER_STATE.take())
            .into_iter()
            .reduce(|mut acc, el| {
                acc.num_tests_run += el.num_tests_run;
                acc.unexpected_results.extend(el.unexpected_results);
                acc
            })
            .unwrap()
    }

    /// If `filters` is empty, every test in this directory and its descendants
    /// should be run. Otherwise, only tests which have one of the entries of
    /// `filters` as its prefix (considering their relative path to the `path`
    /// directory) will be run.
    fn walk_dir(&self, path: &PathBuf, filters: &TestFilters) {
        // Iterate through every entry in this directory in parallel.
        read_dir(path).unwrap().par_bridge().for_each(|entry| {
            let entry = entry.unwrap();

            if entry.file_type().unwrap().is_dir() {
                if let Some(child_filters) = filters.filter_dir(&entry.file_name()) {
                    self.walk_dir(&entry.path(), &child_filters);
                }
            }

            if entry.file_type().unwrap().is_file() && filters.filter_file(&entry.file_name()) {
                self.run_test(&entry.path());
            }
        })
    }

    fn run_test(&self, path: &PathBuf) {
        let Some(test_result) = self.inner.run_test(path) else {
            return;
        };

        RUNNER_STATE.with_borrow_mut(|state| state.num_tests_run += 1);

        let relpath = path.strip_prefix(&self.inner.tests_base).unwrap();

        let expectation = self
            .expectations
            .get(relpath)
            .copied()
            .unwrap_or(TestExpectation::Pass);

        if test_result != expectation {
            // If we're treating crashes as failures, ignore any mismatch where
            // one side is a crash and the other a fail.
            if self.treat_crashes_as_failures
                && matches!(test_result, TestExpectation::Fail | TestExpectation::Crash)
                && matches!(expectation, TestExpectation::Fail | TestExpectation::Crash)
            {
                return;
            }

            RUNNER_STATE.with_borrow_mut(|state| {
                state
                    .unexpected_results
                    .insert(relpath.to_path_buf(), test_result)
            });
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
                // ErrorKind::Interrupted errors are non-fatal, and the
                // operation should be retried.
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
                    if idx >= YAML_END.len() {
                        bytes.extend_from_slice(&read_slice[..idx - YAML_END.len()]);
                    } else {
                        // If idx < YAML_END.len(), the start of the YAML_END
                        // was in the previous chunk, so we need to remove
                        // already-consumed bytes.
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
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommands>,

    #[command(flatten)]
    run_tests: RunTestsArgs,
}

#[derive(Debug, Subcommand)]
enum CliCommands {
    EvalTest { path: PathBuf },
}

#[derive(Debug, Args)]
struct RunTestsArgs {
    #[arg(short = 'j', long)]
    num_threads: Option<NonZeroUsize>,

    /// Update the expectations file with the results of the test run.
    #[arg(short, long)]
    update_expectations: bool,

    /// If true, crashes and failures are not distinguished as test
    /// expectations, i.e. if a test should crash in the expectation file but it
    /// fails instead, that is not counted as a test failure. True by default.
    #[arg(
        long = "dont-treat-crashes-as-failures",
        action = clap::ArgAction::SetFalse,
        help = "Don't treat failures as valid for crash test expectations and vice versa"
    )]
    treat_crashes_as_failures: bool,

    #[arg(short, long)]
    /// Don't print progress messages
    noprogress: bool,

    /// Filters to apply to the tests to run.
    ///
    /// Can be absolute paths, or relative to the test folder.
    filters: Vec<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

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

    let base_runner = BaseTest262Runner {
        runner_base_path,
        tests_base,
        nova_harness_path,
        nova_cli_path,
        print_progress: false,
        in_test_eval: false,
    };

    match cli.command {
        Some(CliCommands::EvalTest { path }) => eval_test(base_runner, path),
        None => run_tests(base_runner, cli.run_tests),
    }
}

fn eval_test(mut base_runner: BaseTest262Runner, path: PathBuf) {
    base_runner.print_progress = false;
    base_runner.in_test_eval = true;

    let canonical_path = base_runner.tests_base.join(&path).canonicalize().unwrap();
    assert!(canonical_path.is_absolute());

    if !canonical_path.starts_with(&base_runner.tests_base)
        || !is_test_file(canonical_path.file_name().unwrap().to_str().unwrap())
    {
        eprintln!("{:?} is not a valid test file", canonical_path);
        std::process::exit(1);
    }

    let Some(result) = base_runner.run_test(&canonical_path) else {
        eprintln!(
            "{:?} is a module or async test, which aren't yet supported by the test runner.",
            path
        );
        std::process::exit(1);
    };

    println!();
    println!("Test result: {:?}", result);
    if result != TestExpectation::Pass {
        std::process::exit(1);
    }
}

fn run_tests(mut base_runner: BaseTest262Runner, args: RunTestsArgs) {
    base_runner.print_progress = !args.noprogress;
    base_runner.in_test_eval = false;

    let expectation_path = base_runner.runner_base_path.join("expectations.json");
    let expectations = {
        if args.update_expectations && !expectation_path.is_file() {
            Default::default()
        } else {
            let file = File::open(&expectation_path).unwrap();
            if args.update_expectations && file.metadata().unwrap().len() == 0 {
                Default::default()
            } else {
                let read_result = serde_json::from_reader(&file);
                if args.update_expectations
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

    let mut filters = TestFilters {
        allowlist: vec![],
        denylist: SKIP_DIRS.iter().map(PathBuf::from).collect(),
    };

    // Skip tests (skip.json)
    {
        #[derive(Deserialize)]
        struct SkipJson {
            skip: Vec<PathBuf>,
        }
        let skipped_tests_path = base_runner.runner_base_path.join("skip.json");
        let file = File::open(&skipped_tests_path).unwrap();
        let skip_json: SkipJson = serde_json::from_reader(&file).unwrap();
        filters.denylist.extend(skip_json.skip);
    }

    // Preprocess filters
    let mut filters_are_valid = true;
    filters
        .allowlist
        .extend(args.filters.into_iter().map_while(|filter| {
            let absolute = base_runner.tests_base.join(filter);
            assert!(absolute.is_absolute());
            let Ok(canonical) = absolute.canonicalize() else {
                filters_are_valid = false;
                return None;
            };
            assert!(canonical.is_absolute());
            let Ok(relative) = canonical.strip_prefix(&base_runner.tests_base) else {
                filters_are_valid = false;
                return None;
            };
            Some(relative.to_path_buf())
        }));

    let runner = Test262Runner {
        inner: base_runner,
        treat_crashes_as_failures: args.treat_crashes_as_failures,
        expectations,
    };
    let run_result = if filters_are_valid {
        runner.run(&filters, args.num_threads)
    } else {
        Default::default()
    };

    if run_result.num_tests_run == 0 {
        println!("No tests found. Check your filters.");
        std::process::exit(1);
    }

    if run_result.unexpected_results.is_empty() {
        println!("No unexpected test results");
    } else if !args.update_expectations {
        println!(
            "Found {} unexpected test results:",
            run_result.unexpected_results.len()
        );
        for (path, result) in &run_result.unexpected_results {
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
            run_result.unexpected_results.len()
        );

        let mut expectations = runner.expectations;
        for (path, result) in run_result.unexpected_results {
            if result == TestExpectation::Pass {
                expectations.remove(&path);
            } else {
                expectations.insert(path, result);
            }
        }

        // We convert to a JSON value first because that way the paths are
        // ordered alphabetically.
        // See https://stackoverflow.com/questions/67789198.
        let json = serde_json::to_value(expectations).unwrap();
        let mut file = File::create(expectation_path).unwrap();
        serde_json::to_writer_pretty(&mut file, &json).unwrap();
    }
}
