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
    |name: String| {
        let mut it = path.iter();
        while Some(str_) = it.next() {
            let path = PathBuf::from(str_).push(name).set_extension("so");
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

/// example:
/// let cache = Cache.new(load_lib("/path/to/lib"));
/// cache.get("myawesomebot").fun(message);
