"""Execute the actual game-side shutdown adapter, including its error-result path."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ServerStoppedTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('rustc'), 'Rust toolchain is required')
    def test_failed_run_keeps_its_error_and_does_not_report_success(self):
        source = (ROOT / 'mod/pluto-server-stopped.rs').as_posix()
        fixture = f'''type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[path = r"{source}"]
mod adapter;
static EVENTS: std::sync::Mutex<Vec<bool>> = std::sync::Mutex::new(Vec::new());
#[test]
fn completed_only_on_success() {{
    adapter::set_dispatcher(|dedicated| EVENTS.lock().unwrap().push(dedicated));
    let failed = adapter::finish(Err("final save failed".into()), false);
    assert_eq!(failed.unwrap_err().to_string(), "final save failed");
    assert!(EVENTS.lock().unwrap().is_empty());
    assert!(adapter::finish(Ok(()), false).is_ok());
    assert!(adapter::finish(Ok(()), true).is_ok());
    assert_eq!(*EVENTS.lock().unwrap(), [false, true]);
}}
'''
        with tempfile.TemporaryDirectory(prefix='pluto-stopped-') as temp:
            temp = Path(temp)
            test = temp / 'test.rs'
            binary = temp / ('test.exe' if os.name == 'nt' else 'test')
            test.write_text(fixture, encoding='utf-8', newline='\n')
            subprocess.run(['rustc', '--edition=2024', '--test', str(test), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
