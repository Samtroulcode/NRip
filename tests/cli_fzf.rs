use serial_test::serial;
use which::which;

#[test]
#[serial]
fn list_works_without_fzf() {
    // Force désactivation via env/flag selon ton implémentation
}

#[test]
#[serial]
fn list_uses_fzf_when_available() {
    if which("fzf").is_err() {
        eprintln!("skip: fzf not found");
        return;
    }
}
