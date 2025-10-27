use std::collections::HashMap;

pub struct Context {}

type Handler = fn(&[String], &mut Context);

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

        Self { map }
    }

    pub fn dispatch(&self, name: &str, argv: &[String], context: &mut Context) {
        if let Some(handler) = self.map.get(name) {
            handler(argv, ctx);
        } else {
            eprintln!("Unknown command: {name}");
        }
    }
}

pub mod command1;
