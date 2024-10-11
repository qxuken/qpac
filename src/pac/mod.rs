use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use sha2::Digest;

#[derive(Debug)]
pub struct Pac {
    pub file: String,
    pub hash: String,
}

const JS_SCRIPT: &str = include_str!("./pac.js");

impl Pac {
    pub fn new(file: String, hash: String) -> Self {
        Self { file, hash }
    }

    /// `hosts` should be sorted for binary search in a pac file
    pub fn generate(hosts: Vec<String>) -> Self {
        let hosts_bytes: usize = hosts.iter().map(|h| h.len()).sum();
        let mut hasher = sha2::Sha512::new();
        let mut file =
            String::with_capacity(18 + 3 + JS_SCRIPT.len() + hosts_bytes + hosts.len() * 3);
        file.push_str("var __HOSTS__ = [");
        for host in hosts.into_iter() {
            let s = format!(r#""{host}","#);
            file.push_str(&s);
            hasher.update(s.as_bytes());
        }
        if hosts_bytes > 0 {
            file.pop();
        }
        file.push_str("];\n");
        file.push_str(r#"var __PROXY__ = "SOCKS5 127.0.0.1:1080; SOCKS 127.0.0.1:1080; DIRECT;""#);
        file.push('\n');
        file.push_str(JS_SCRIPT);
        let hash = URL_SAFE.encode(hasher.finalize()).to_string();
        Pac { file, hash }
    }
}
