use std::collections::HashMap;

#[derive(Debug)]
pub struct Args {
    binary: String,
    positional: Vec<String>,
    options: HashMap<String, Option<String>>,
}

impl Args {
    pub fn new(args: Vec<String>) -> Self {
        let mut binary: String = String::new();
        let mut positional = Vec::new();
        let mut options = HashMap::new();

        let mut iter = args.iter().peekable();
        let flag_prefix = "--";
        let mut parent_flag: Option<String> = None;

        while iter.peek().is_some() {
            let arg = iter.next().unwrap();

            if binary.is_empty() {
                binary.clone_from(arg);
                continue;
            }

            if let Some(parent) = &parent_flag {
                options.insert(parent.clone(), Some(arg.clone()));
                parent_flag = None;
                continue;
            }

            if !arg.starts_with(flag_prefix) {
                positional.push(arg.clone());
                continue;
            }

            let flag = arg.trim_start_matches(flag_prefix);

            if let Some((key, value)) = flag.split_once('=') {
                options.insert(key.to_string(), Some(value.to_string()));
                continue;
            } else {
                parent_flag.replace(flag.to_string());
                options.insert(flag.to_string(), None);
            }
        }

        Self {
            binary,
            positional,
            options,
        }
    }

    #[allow(dead_code)]
    pub fn binary(&self) -> &String {
        &self.binary
    }

    pub fn positional(&self, index: usize) -> Option<&String> {
        self.positional.get(index)
    }

    pub fn option(&self, key: &str) -> Option<String> {
        self.options.get(key).and_then(|v| v.clone())
    }

    #[allow(dead_code)]
    pub fn has_option(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }
}

impl<T: Iterator<Item = String>> From<T> for Args {
    fn from(iter: T) -> Self {
        Self::new(iter.collect())
    }
}
