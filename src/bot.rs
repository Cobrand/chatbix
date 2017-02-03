use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

extern crate libloading as lib;

pub struct Bot {
    pub handler: lib::Library;
    pub fun: lib::Symbol<unsafe extern fn(str) -> str>;
}

pub struct Cache {
    cache: HashMap<String, Bot>,
    loader: fn(string) -> Bot
}

impl Cache {
    pub fn new(loader_: fn(String) -> Bot) -> Cache {
        Cache {
            loader: loader_,
            cache: HashMap::new()
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

pub fn load_lib(path: Vec<String>) -> (fn(String) -> Option<Bot>) {
    |name: string| {
        let mut it = path.iter();
        while Some(str_) = it.next() {
            let path = PathBuf::from(str_).push(name);
            if path.as_path().exists() {
                let lib = try!(lib::Library::new(path));
                unsafe {
                    let fun_ = try!(lib.get(b"parse_msg"));
                    return Some(Bot {handler: lib, fun: fun_})
                }
            }
        }
        None
    }
}
