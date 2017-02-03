use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

extern crate libloading as lib;

pub struct Bot<'a> {
    pub handler: lib::Library,
    pub fun: lib::Symbol<'a, unsafe extern "C" fn(*const char) -> *const char>,
}

pub struct Cache<'a> {
    cache: HashMap<String, Bot<'a>>,
    path: Vec<String>,
}

impl<'a> Cache<'a> {
    pub fn new(path: Vec<String>) -> Cache<'a> {
        Cache {
            path: path,
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, name: String) -> Bot {
        match self.cache.get(name) {
            Some(bot) => bot,
            None => {
                match self.loader(name) {
                    Some(fun) => {
                        self.cache.insert(name, self.loader(name));
                        self.get(name)
                    },
                    None => None
                }
            }
        }
    }
}

pub fn load_lib<'a, S: AsRef<str>>(path: &Vec<String>, name: S) -> Option<Bot<'a>> {
    let mut it = path.iter();
    while let Some(str_) = it.next() {
        let mut path = PathBuf::from(str_);
        path.push(name.as_ref());
        path.set_extension("so");
        if path.as_path().exists() {
            let lib = match lib::Library::new(path) {
                Ok(l) => l,
                Err(e) => return None
            };
            unsafe {
                let fun_ = match lib.get(b"parse_msg") {
                    Ok(l) => l,
                    Err(e) => return None
                };
                return Some(Bot {
                    handler: lib,
                    fun: fun_,
                });
            }
        }
    }
    None
}

/// example:
/// let cache = Cache.new(load_lib("/path/to/lib"));
/// cache.get("myawesomebot").fun(message);
