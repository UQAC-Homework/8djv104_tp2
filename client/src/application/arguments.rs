/// Structure used to store the arguments of the application
pub struct Arguments {
    /// Network address of the server
    pub server_ip: String,

    /// Network port of the server
    pub server_port: u16,

    /// Username of the user
    pub username: String,

    /// Defines if network logs are shown
    pub verbose: bool,

    /// Percentage of chance that a packet is dropped
    pub packet_drop_rate: f32,
}

impl Arguments {
    pub fn new<I>(args: I) -> Result<Self, Option<String>>
    where
        I: Iterator<Item = String>,
    {
        let mut verbose = false;
        let mut packet_drop_rate: f32 = 0.0;
        let mut parse_packet_drop_rate = false;
        let mut positional = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-v" | "--verbose" => verbose = true,
                "-h" | "--help" => return Err(None),
                "--loss" => parse_packet_drop_rate = true,
                _ => {
                    if arg.starts_with('-') && arg.len() > 1 {
                        return Err(Some(format!("Unknown option: `{arg}`")));
                    }

                    if parse_packet_drop_rate {
                        parse_packet_drop_rate = false;

                        let rate_conversion = arg.parse::<f32>();

                        if rate_conversion.is_err() {
                            return Err(Some(format!("Invalid loss probability: {}", arg)));
                        }

                        packet_drop_rate = rate_conversion.unwrap();
                    } else {
                        positional.push(arg)
                    }
                }
            }
        }

        let mut username = "Player";
        let mut server_address = "127.0.0.1";
        let mut server_port: u16 = 9999;

        if positional.len() > 0 {
            username = positional.get(0).unwrap();
        }

        if positional.len() > 1 {
            server_address = positional.get(1).unwrap();
        }

        if positional.len() > 2 {
            let raw_port = positional.get(2).unwrap();
            let port_conversion = raw_port.parse::<u16>();

            if port_conversion.is_err() {
                return Err(Some(format!("Invalid port number: {}", raw_port)));
            }

            server_port = port_conversion.unwrap();
        }

        Ok(Arguments {
            server_ip: server_address.parse().unwrap(),
            server_port,
            username: username.parse().unwrap(),
            verbose,
            packet_drop_rate,
        })
    }
}
