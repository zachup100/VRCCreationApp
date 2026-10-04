#[cfg(windows)]
mod platform {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const WEBVIEW2_CLIENT_KEY: &str =
        r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    const WEBVIEW2_DOWNLOAD_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

    fn has_version(root: winreg::HKEY, subkey: &str) -> bool {
        RegKey::predef(root)
            .open_subkey(subkey)
            .and_then(|key| key.get_value::<String, _>("pv"))
            .map(|version| !version.is_empty() && version != "0.0.0.0")
            .unwrap_or(false)
    }

    fn webview2_installed() -> bool {
        has_version(
            HKEY_LOCAL_MACHINE,
            &format!(r"SOFTWARE\WOW6432Node\{WEBVIEW2_CLIENT_KEY}"),
        ) || has_version(
            HKEY_LOCAL_MACHINE,
            &format!(r"SOFTWARE\{WEBVIEW2_CLIENT_KEY}"),
        ) || has_version(
            HKEY_CURRENT_USER,
            &format!(r"Software\{WEBVIEW2_CLIENT_KEY}"),
        )
    }

    pub fn ensure() -> bool {
        if webview2_installed() {
            return true;
        }
        let open_download = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Microsoft WebView2 Runtime required")
            .set_description(
                "This app needs the Microsoft WebView2 Runtime, which was not found on this computer.\n\nOpen the official Microsoft download page now?",
            )
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes;
        if open_download {
            let _ = open::that_detached(WEBVIEW2_DOWNLOAD_URL);
        }
        false
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn ensure() -> bool {
        true
    }
}

pub fn ensure() -> bool {
    platform::ensure()
}
