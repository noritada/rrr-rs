#[cfg(unix)]
use {pager::Pager, which::which};

#[cfg(unix)]
pub fn start_pager() {
    if which("less").is_ok() {
        Pager::with_pager("less -R").setup();
    } else {
        Pager::new().setup();
    }
}

#[cfg(not(unix))]
pub fn start_pager() {}
