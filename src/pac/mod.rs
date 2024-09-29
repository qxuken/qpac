#[derive(Debug)]
pub struct Pac {
    file: String,
}

const JS_SCRIPT: &str = include_str!("./pac.js");

impl Pac {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            file: Self::generate(hosts),
        }
    }

    fn generate(hosts: Vec<String>) -> String {
        let hosts_bytes: usize = hosts.iter().map(|h| h.len()).sum();
        let mut res = String::with_capacity(18 + 3 + JS_SCRIPT.len() + hosts_bytes);
        res.push_str("var __HOSTS__ = new Set([\n");
        for host in hosts.into_iter() {
            res.push_str(&format!(r#""{host}","#));
        }
        res.push_str("]);\n");
        res.push_str(JS_SCRIPT);
        res
    }
}

impl Pac {
    pub fn get_file(&self) -> String {
        self.file.clone()
    }

    pub fn update(&mut self, hosts: Vec<String>) {
        self.file = Self::generate(hosts);
    }
}
