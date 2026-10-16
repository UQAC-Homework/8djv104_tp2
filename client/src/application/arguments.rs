/// Structure used to store the arguments of the application
pub struct Arguments {
    /// Network address of the server
    server_address: String,

    /// Username of the user
    username: String,

    /// Defines if network logs are shown
    verbose: bool,
}

// TODO: Add default values
// TODO: Reorder arguments

impl Arguments {
    pub fn new<I>(args: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let mut verbose = false;
        let mut positional = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-v" | "--verbose" => verbose = true,
                "-h" | "--help" => return Err(String::new()),
                _ if arg.starts_with('-') && arg.len() > 1 => {
                    return Err(format!("unknown option `{arg}`"));
                }
                _ => positional.push(arg),
            }
        }

        match <[String; 2]>::try_from(positional) {
            Ok([server, username]) => Ok(Arguments {
                server_address: server,
                username,
                verbose,
            }),
            Err(_) => Err("expected a server address and a username".to_string()),
        }
    }
}
