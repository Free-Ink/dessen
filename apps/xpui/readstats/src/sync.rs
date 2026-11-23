//! Importing the reading history kept on CrossPoint Sync.

use crate::bytes::Bytes;
use crate::json;
use crate::log::LOG;

/// The firmware's sync settings (crosspoint-reader
/// `lib/KOReaderSync/KOReaderCredentialStore`): the server and username are
/// plain JSON there; the password is not, so the reader types it.
const SETTINGS: &[u8] = b"/.crosspoint/koreader.json";
const DEFAULT_SERVER: &[u8] = b"https://sync.crosspointreader.com";

/// The CrossPoint Sync account set up on this reader.
pub struct Account {
    /// The server's base URL, without a trailing slash.
    server: Bytes<256>,
    user: Bytes<64>,
}

impl Account {
    /// The account in the firmware's settings; `None` without one.
    #[inline(never)] // its buffers stay off the caller's frame
    pub fn load() -> Option<Account> {
        let mut settings = [0u8; 2048];
        let len = dessen::dsn_fs_read(SETTINGS, &mut settings);
        if len <= 0 {
            return None;
        }
        let settings = &settings[..(len as usize).min(settings.len())];
        let mut user = [0u8; 64];
        let user_len =
            json::string(settings, b"\"username\":", &mut user).filter(|len| *len > 0)?;
        let mut url = [0u8; 200];
        let url_len = json::string(settings, b"\"serverUrl\":", &mut url).unwrap_or(0);
        let url = &url[..url_len];
        // KOReaderCredentialStore::getBaseUrl: the default server, http://
        // when there is no scheme, no trailing slash.
        let mut server = Bytes::new();
        let fits = match url {
            [] => server.push(DEFAULT_SERVER),
            url if url.windows(3).any(|window| window == b"://") => server.push(url),
            url => server.push_all(&[b"http://", url]),
        };
        server.trim_end(b'/');
        let mut account = Account {
            server,
            user: Bytes::new(),
        };
        (fits && account.user.push(&user[..user_len])).then_some(account)
    }

    /// Downloads the account's history (crosspoint-sync
    /// `GET /api/v1/stats/sessions/export`) into the empty log, dated in the
    /// zone `offset_minutes` behind UTC. `show` puts a progress line on
    /// screen. The error is what to tell the reader; it is empty when the
    /// password prompt was cancelled.
    #[inline(never)] // its buffers stay off the caller's frame
    pub fn import(&self, offset_minutes: i32, show: &dyn Fn(&str)) -> Result<(), &'static str> {
        let (user, server) = (self.user.as_slice(), self.server.as_slice());
        let mut prompt = Bytes::<100>::new();
        prompt.push_all(&[b"CrossPoint Sync password for ", user]);
        let mut password = [0u8; 128];
        let len = dessen::dsn_text_input(prompt.as_slice(), &mut password);
        if len <= 0 {
            return Err("");
        }
        let password = &password[..(len as usize).min(password.len())];
        if password.iter().any(|byte| *byte < 32 || *byte == 127) {
            return Err("That password can't be sent from here.");
        }
        show("Connecting...");
        if dessen::dsn_wifi_connect() < 0 {
            return Err("Couldn't connect to Wi-Fi.");
        }
        // The server takes the typed password as the key, like other kosync
        // clients that send it.
        let zone = kit::join!(offset_minutes);
        let mut head = Bytes::<1024>::new();
        let fits = head.push_all(&[
            b"GET ",
            server,
            b"/api/v1/stats/sessions/export-link?tz=",
            zone.as_bytes(),
            b"\nx-auth-user: ",
            user,
            b"\nx-auth-key: ",
            password,
            b"\n",
        ]);
        if !fits {
            return Err("These sync settings are too long to use here.");
        }
        let mut response = [0u8; 1024];
        let len = dessen::dsn_net_request(head.as_slice(), b"", &mut response);
        if len < 0 {
            return Err("Couldn't reach CrossPoint Sync.");
        }
        // The response: u16 status and u16 header length, big-endian, the
        // headers, then the body.
        let status = u16::from_be_bytes([response[0], response[1]]);
        if status == 401 {
            return Err("Wrong password.");
        }
        let headers = usize::from(u16::from_be_bytes([response[2], response[3]]));
        let body_start = (4 + headers).min(response.len());
        let body = &response[body_start..(body_start + len as usize).min(response.len())];
        let mut path = [0u8; 512];
        let mut url = Bytes::<768>::new();
        match json::string(body, b"\"path\":", &mut path) {
            Some(path_len)
                if status == 200 && path_len > 0 && url.push_all(&[server, &path[..path_len]]) => {}
            _ => return Err("This sync server can't send reading history yet."),
        }
        if json::after(body, b"\"sessions\":").is_some()
            && json::number(body, b"\"sessions\":") == 0
        {
            return Err("There's no reading history on CrossPoint Sync to import yet.");
        }
        let estimated = json::after(body, b"\"estimated\":true").is_some();
        show(if estimated {
            "Downloading your reading history. Time for days synced without it is estimated from pages."
        } else {
            "Downloading your reading history..."
        });
        // The log is missing or empty here, so writing it whole loses nothing.
        if dessen::dsn_fetch_to_sd(url.as_slice(), LOG) < 0 {
            return Err("The download didn't finish. Try again.");
        }
        Ok(())
    }
}
