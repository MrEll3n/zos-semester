use std::collections::HashMap;

pub struct Context {}

type Handler = fn(&[&str], &mut Context);

pub struct Registry {
    pub map: HashMap<&'static str, Handler>,
}

impl Registry {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert(
            "command1",
            crate::commands::command1::handle_argv as Handler,
        );
        map.insert(
            "exit",
            crate::commands::exit::handle_argv as Handler,
        );

        Self { map }
    }

    pub fn dispatch(&self, name: &str, argv: &[&str], context: &mut Context) {
        if let Some(handler) = self.map.get(name) {
            handler(argv, context);
        } else {
            eprintln!("Unknown command: {name}");
        }
    }
}

pub mod command1;
pub mod exit;
