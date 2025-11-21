// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::{Args, Parser as ClapParser, Subcommand, builder::ArgPredicate};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    ffi::OsStr,
    fs::{File, read_dir},
    io::{ErrorKind, Read, Write},
    num::NonZeroUsize,
    path::{PathBuf, absolute},
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum TestExpectation {
    Pass,
    Fail,
    Unresolved,
    Crash,
    Timeout,
}

fn is_test_file(file_name: &str) -> bool {
    // File names containing the string "_FIXTURE" are JS modules which get
    // imported by module tests. They should not be run as standalone tests.
    // See https://github.com/tc39/test262/blob/main/INTERPRETING.md#modules
    file_name.ends_with(".js") && !file_name.contains("_FIXTURE")
}

fn count_test_files(path: PathBuf) -> usize {
    if let Ok(dir) = read_dir(&path) {
        dir.fold(0, |count, entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_ok_and(|e| e.is_file())
                    && entry.file_name().to_str().is_some_and(is_test_file)
                {
                    count + 1
                } else {
                    count + count_test_files(entry.path())
                }
            } else {
                count
            }
        })
    } else if path.is_file()
        && path
            .file_name()
            .and_then(|path| path.to_str())
            .is_some_and(is_test_file)
    {
        1
    } else {
        0
    }
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
        if let Some(str_file_name) = os_file_name.to_str()
            && !is_test_file(str_file_name)
        {
            return false;
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
    run_gc: bool,
}

impl BaseTest262Runner {
    const TEST_TIMEOUT: Duration = Duration::from_secs(10);

    /// This error code denotes a parse error or an uncaught exception, all
    /// others are treated as crashes.
    const FAILURE_ERROR_CODE: i32 = 1;

    fn run_test(&self, path: &PathBuf) -> TestExpectation {
        let metadata = test_metadata::parse(path);

        if self.print_progress {
            let relpath = path.strip_prefix(&self.tests_base).unwrap();
            let mut message = format!("Running {}", relpath.to_string_lossy());
            if message.len() > 80 {
                message.truncate(80 - 3);
                message.push_str("...");
            }
            // These escape codes make this line overwrite the previous line.
            print!("{message}\x1B[0K\r");
        }

        if metadata.flags.is_async {
            assert!(
                metadata.negative.is_none(),
                "Unexpected negative async expectation in {path:?}",
            );
        }

        let mut modes_run = 0;
        for strict in [false, true] {
            if (metadata.flags.raw && strict) || metadata.flags.strict == Some(!strict) {
                continue;
            }

            modes_run += 1;

            let mut command = Command::new(&self.nova_cli_path);
            command.arg("eval");
            command.arg("--expose-internals");
            if metadata.flags.module {
                command.arg("--module");
            }
            if metadata.flags.can_block == Some(false) {
                command.arg("--no-block");
            }
            if !strict {
                command.arg("--no-strict");
            }
            if !self.run_gc {
                command.arg("--nogc");
            }

            command.arg(&self.nova_harness_path);
            if metadata.flags.raw {
                assert!(metadata.includes.is_empty());
                assert!(!metadata.flags.is_async);
            } else {
                let async_include = if metadata.flags.is_async {
                    Some("doneprintHandle.js")
                } else {
                    None
                };
                let auto_includes = [Some("assert.js"), Some("sta.js"), async_include];

                let mut harness = self.runner_base_path.clone();
                harness.push("test262");
                harness.push("harness");
                let includes_iter = auto_includes
                    .iter()
                    .filter_map(|include| *include)
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
                if strict {
                    println!("Strict mode run:");
                } else {
                    println!("Loose mode run:")
                }
                println!("Running: {command:?}");
                println!();
            }

            let expectation = self.run_command_and_parse_output(
                command,
                &metadata.negative,
                metadata.flags.is_async,
            );
            if self.in_test_eval {
                println!();
                println!("Test result: {expectation:?}");
                if !strict {
                    println!();
                }
            }

            if expectation != TestExpectation::Pass {
                return expectation;
            }
        }

        // Make sure all tests ran at least one mode (strict or loose).
        assert_ne!(modes_run, 0);

        TestExpectation::Pass
    }

    fn run_command_and_parse_output(
        &self,
        mut command: Command,
        negative: &Option<test_metadata::NegativeExpectation>,
        is_async: bool,
    ) -> TestExpectation {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

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

        let result = if status.success() {
            if is_async {
                let pass_prefix = "Test262:AsyncTestComplete\n";
                let fail_prefix = "Test262:AsyncTestFailure:";

                let mut buffer = vec![0u8; pass_prefix.len().max(fail_prefix.len())];
                let buffer = {
                    let mut read = 0;
                    while read < buffer.len() {
                        match child.stdout.as_mut().unwrap().read(&mut buffer[read..]) {
                            Ok(0) => break,
                            Ok(bytes_read) => read += bytes_read,
                            Err(e) if e.kind() == ErrorKind::Interrupted => {}
                            Err(e) => panic!("{e:?}"),
                        }
                    }
                    &buffer[..read]
                };

                if self.in_test_eval {
                    std::io::stdout().write_all(buffer).unwrap();
                }

                if buffer.starts_with(pass_prefix.as_bytes()) {
                    TestExpectation::Pass
                } else if buffer.starts_with(fail_prefix.as_bytes()) {
                    TestExpectation::Fail
                } else {
                    TestExpectation::Unresolved
                }
            } else if negative.is_none() {
                TestExpectation::Pass
            } else {
                TestExpectation::Fail
            }
        } else if let Some(negative) = negative {
            let expected_stderr_prefix: Cow<str> = match negative.phase {
                test_metadata::TestFailurePhase::Parse => "Parse errors:".into(),
                test_metadata::TestFailurePhase::Runtime => {
                    format!("Uncaught exception: {}", negative.error_type).into()
                }
                test_metadata::TestFailurePhase::Resolution => {
                    format!("Unresolved: {}", negative.error_type).into()
                }
            };

            let mut buffer = vec![0u8; expected_stderr_prefix.len()];
            match child.stderr.as_mut().unwrap().read_exact(&mut buffer) {
                Ok(_) => {
                    if self.in_test_eval {
                        std::io::stderr().write_all(&buffer).unwrap();
                    }
                    if buffer == expected_stderr_prefix.as_bytes() {
                        TestExpectation::Pass
                    } else {
                        TestExpectation::Fail
                    }
                }
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        TestExpectation::Fail
                    } else {
                        panic!("{e:?}");
                    }
                }
            }
        } else {
            TestExpectation::Fail
        };

        if self.in_test_eval {
            std::io::copy(&mut child.stdout.unwrap(), &mut std::io::stdout()).unwrap();
            std::io::stdout().flush().unwrap();
            std::io::copy(&mut child.stderr.unwrap(), &mut std::io::stderr()).unwrap();
            std::io::stderr().flush().unwrap();
        }

        result
    }
}

#[derive(Debug)]
struct Test262Runner {
    inner: BaseTest262Runner,
    expectations: HashMap<PathBuf, TestExpectation>,
}

#[derive(Debug, Default)]
struct Test262RunnerState {
    unexpected_results: HashMap<PathBuf, TestExpectation>,
    num_tests_run: usize,
    num_tests_pass: usize,
    num_tests_fail: usize,
    num_tests_unresolved: usize,
    num_tests_crash: usize,
    num_tests_timeout: usize,
}

#[derive(Serialize, Deserialize)]
struct Test262RunnerMetrics {
    total: usize,
    results: Test262RunnerMetricsResults,
}

#[derive(Serialize, Deserialize)]
struct Test262RunnerMetricsResults {
    pass: usize,
    fail: usize,
    unresolved: usize,
    crash: usize,
    timeout: usize,
    skip: usize,
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
                acc.num_tests_pass += el.num_tests_pass;
                acc.num_tests_fail += el.num_tests_fail;
                acc.num_tests_unresolved += el.num_tests_unresolved;
                acc.num_tests_crash += el.num_tests_crash;
                acc.num_tests_timeout += el.num_tests_timeout;
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

            if entry.file_type().unwrap().is_dir()
                && let Some(child_filters) = filters.filter_dir(&entry.file_name())
            {
                self.walk_dir(&entry.path(), &child_filters);
            }

            if entry.file_type().unwrap().is_file() && filters.filter_file(&entry.file_name()) {
                self.run_test(&entry.path());
            }
        })
    }

    fn run_test(&self, path: &PathBuf) {
        let test_result = self.inner.run_test(path);

        RUNNER_STATE.with_borrow_mut(|state| {
            state.num_tests_run += 1;
            match test_result {
                TestExpectation::Pass => state.num_tests_pass += 1,
                TestExpectation::Fail => state.num_tests_fail += 1,
                TestExpectation::Unresolved => state.num_tests_unresolved += 1,
                TestExpectation::Crash => state.num_tests_crash += 1,
                TestExpectation::Timeout => state.num_tests_timeout += 1,
            }
        });

        let relpath = path.strip_prefix(&self.inner.tests_base).unwrap();

        let expectation = self
            .expectations
            .get(relpath)
            .copied()
            .unwrap_or(TestExpectation::Pass);

        if test_result != expectation {
            // In Windows, `relpath` will likely contain backwards slashes,
            // which shouldn't end up in the JSON output, because they will
            // not match in Unix systems. So we replace them with forward
            // slashes before inserting the path into `unexpected_results`.
            let output_path = if cfg!(windows) {
                let mut path_string = relpath.to_str().unwrap().to_string();
                let mut idx = 0;
                while let Some(found_idx) = path_string[idx..].find('\\') {
                    idx += found_idx;
                    path_string.replace_range(idx..(idx + 1), "/");
                    idx += 1;
                }
                PathBuf::from(path_string)
            } else {
                relpath.to_path_buf()
            };

            RUNNER_STATE
                .with_borrow_mut(|state| state.unexpected_results.insert(output_path, test_result));
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

                fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
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
                Err(error) => panic!("{error:?}"),
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
                    Err(error) => panic!("{error:?}"),
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
    EvalTest {
        #[arg(short = 'l', long)]
        /// Run tests that don't require strict mode in loose mode.
        prefer_loose_mode: bool,

        path: PathBuf,
    },
}

#[derive(Debug, Args)]
struct RunTestsArgs {
    #[arg(short = 'j', long)]
    num_threads: Option<NonZeroUsize>,

    /// Updates the expectations and metrics files with the results of the test.
    #[arg(short, long)]
    update: bool,

    /// Update the expectations file with the results of the test run.
    #[arg(
        long,
        default_value_if("update", ArgPredicate::Equals("true".into()), Some("true"))
    )]
    update_expectations: bool,

    /// Update the metrics file with the metrics of the test run.
    #[arg(
        long,
        default_value_if("update", ArgPredicate::Equals("true".into()), Some("true"))
    )]
    update_metrics: bool,

    #[arg(short, long)]
    /// Don't print progress messages
    noprogress: bool,

    #[arg(short = 'l', long)]
    /// Run tests that don't require strict mode in loose mode.
    prefer_loose_mode: bool,

    /// Filters to apply to the tests to run.
    ///
    /// Can be absolute paths, or relative to the test folder.
    filters: Vec<PathBuf>,

    /// Run garbage collection between each script run.
    ///
    /// Garbage collection is currently disabled by default as it needlessly
    /// increases the test runtime by about two-fold.
    #[arg(long)]
    gc: bool,
}

fn main() {
    let cli = Cli::parse();

    // We're expecting this binary to always be run in the same machine at
    // the same time as the repo checkout exists.
    let runner_base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_base = absolute(runner_base_path.join("test262").join("test")).unwrap();
    let nova_harness_path = runner_base_path.join("nova-harness.js");

    let nova_cli_path = {
        // current_exe should be target/release/test262(.exe)
        // The nova_cli binary is target/release/nova_cli(.exe)
        let mut path = std::env::current_exe().unwrap();
        assert!(path.pop());
        path.push("nova_cli");
        if cfg!(windows) {
            path.set_extension("exe");
        }
        assert!(
            path.exists(),
            "nova_cli binary is missing. Please run `cargo build -p nova_cli`"
        );
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
        run_gc: cli.run_tests.gc,
    };

    match cli.command {
        Some(CliCommands::EvalTest { path, .. }) => eval_test(base_runner, path),
        None => run_tests(base_runner, cli.run_tests),
    }
}

fn eval_test(mut base_runner: BaseTest262Runner, path: PathBuf) {
    base_runner.print_progress = false;
    base_runner.in_test_eval = true;

    let canonical_path = absolute(base_runner.tests_base.join(&path)).unwrap();
    assert!(canonical_path.is_absolute());

    if !canonical_path.starts_with(&base_runner.tests_base)
        || !is_test_file(canonical_path.file_name().unwrap().to_str().unwrap())
    {
        eprintln!("{canonical_path:?} is not a valid test file");
        std::process::exit(1);
    }

    let result = base_runner.run_test(&canonical_path);

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

    let metrics_path = base_runner.runner_base_path.join("metrics.json");
    let metrics: Option<Test262RunnerMetrics> = {
        if args.update_metrics && !metrics_path.is_file() {
            None
        } else {
            let file = File::open(&metrics_path).unwrap();
            if args.update_metrics && file.metadata().unwrap().len() == 0 {
                None
            } else {
                serde_json::from_reader(&file).unwrap()
            }
        }
    };

    let mut filters = TestFilters {
        allowlist: vec![],
        denylist: vec![],
    };

    // Skip tests (skip.json)
    {
        #[derive(Deserialize)]
        struct SkipJson {
            skip: Vec<PathBuf>,
        }
        let skipped_tests_path = base_runner.runner_base_path.join("skip.json");
        let file = File::open(skipped_tests_path).unwrap();
        let skip_json: SkipJson = serde_json::from_reader(&file).unwrap();
        filters.denylist.extend(skip_json.skip);
    }

    // Preprocess filters
    let mut filters_are_valid = true;
    filters
        .allowlist
        .extend(args.filters.into_iter().map_while(|filter| {
            let Ok(canonical) = absolute(base_runner.tests_base.join(filter)) else {
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
    let num_tests_skip = filters.denylist.iter().fold(0, |r, filter| {
        let Ok(canonical) = absolute(base_runner.tests_base.join(filter)) else {
            return r;
        };
        assert!(canonical.is_absolute());
        r + count_test_files(canonical)
    });

    let runner = Test262Runner {
        inner: base_runner,
        expectations,
    };
    let run_result = if filters_are_valid {
        runner.run(&filters, args.num_threads)
    } else {
        Default::default()
    };

    let mut metrics_mismatch = false;
    if args.update_metrics {
        let json = serde_json::to_value(Test262RunnerMetrics {
            total: run_result.num_tests_run + num_tests_skip,
            results: Test262RunnerMetricsResults {
                pass: run_result.num_tests_pass,
                fail: run_result.num_tests_fail,
                unresolved: run_result.num_tests_unresolved,
                crash: run_result.num_tests_crash,
                timeout: run_result.num_tests_timeout,
                skip: num_tests_skip,
            },
        })
        .unwrap();
        let mut file = File::create(metrics_path).unwrap();
        serde_json::to_writer_pretty(&mut file, &json).unwrap();
    } else if let Some(metrics) = metrics {
        if run_result.num_tests_run + num_tests_skip != metrics.total {
            println!(
                "Total test count mismatch: {: >5} vs {: >5}",
                run_result.num_tests_run, metrics.total
            );
            metrics_mismatch = true;
        }
        if run_result.num_tests_pass != metrics.results.pass {
            println!(
                "Pass count mismatch:       {: >5} vs {: >5}",
                run_result.num_tests_pass, metrics.results.pass
            );
            metrics_mismatch = true;
        }
        if run_result.num_tests_fail != metrics.results.fail {
            println!(
                "Fail count mismatch:       {: >5} vs {: >5}",
                run_result.num_tests_fail, metrics.results.fail
            );
            metrics_mismatch = true;
        }
        if run_result.num_tests_unresolved != metrics.results.unresolved {
            println!(
                "Unresolved count mismatch: {: >5} vs {: >5}",
                run_result.num_tests_unresolved, metrics.results.unresolved
            );
            metrics_mismatch = true;
        }
        if run_result.num_tests_crash != metrics.results.crash {
            println!(
                "Crash count mismatch:      {: >5} vs {: >5}",
                run_result.num_tests_crash, metrics.results.crash
            );
            metrics_mismatch = true;
        }
        if run_result.num_tests_timeout != metrics.results.timeout {
            println!(
                "Timeout count mismatch:    {: >5} vs {: >5}",
                run_result.num_tests_timeout, metrics.results.timeout
            );
            metrics_mismatch = true;
        }

        if metrics_mismatch {
            println!("                         (found) vs (expected)");
            println!("Metrics mismatch detected. Please update the metrics file.");
        }
    }

    if run_result.num_tests_run == 0 {
        println!("No tests found. Check your filters.");
        std::process::exit(1);
    }

    if run_result.unexpected_results.is_empty() {
        if !metrics_mismatch {
            println!("No unexpected test results");
        }
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
            println!("\t{path:?} -- Expected {expectation:?}, got {result:?}",);
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

    // TODO: Figure out why metrics mismatch between local and CI. For now we
    // disable erroring out on metrics mismatch.
    // if metrics_mismatch {
    //     std::process::exit(1);
    // }
}
