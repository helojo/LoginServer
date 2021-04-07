use std::path::PathBuf;
use std::io::Write;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppData {
    pub database:       Database,
    pub environment:    Environment
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Environment {
    pub mysql_host:     String,
    pub mysql_database: String,
    pub mysql_username: String,
    pub mysql_password: String
}

#[derive(Clone)]
pub struct Database {
    pub pool:       mysql::Pool
}

impl AppData {
    pub fn new(database: Database, environment: Environment) -> AppData {
        AppData {
            database,
            environment
        }
    }
}

impl Environment {
    pub fn new() -> Environment {
        //Check if the 'USE_ENVIRONMENTAL_VARIABLES' variable is set
        let use_env_vars_wrapped = std::env::var("USE_ENVIRONMENTAL_VARIABLES");
        if use_env_vars_wrapped.is_err() {
            //It isn't set (hence it returns Err), so we read from File
            return Self::get_environment_from_file();
        }

        //If, and only if, the value of the variable is equal to 'TRUE' will we use environmental variables
        if use_env_vars_wrapped.unwrap().eq("TRUE") {
            return Self::get_environment_from_vars();
        }

        //Default to reading from a file
        Self::get_environment_from_file()
    }

    fn get_environment_from_file() -> Environment {
        //Determine the platform, and thus the location of the config file
        //Windows:          C:\Program Files\TwinsightContentDashboard\config.yml
        //Linux/Freebsd:    /etc/twinsight-content-dashboard/config.yml
        //Default:          The same directory as the executable
        let config_path = match std::env::consts::OS {
            "windows" => PathBuf::from(r"C:\Program Files\TwinsightContentDasboard\config.yml"),
            "linux" | "freebsd" => PathBuf::from(r"/etc/twinsight-content-dashboard"),
            _ => {
                eprintln!("Warning! This platform is not officially supported. Your configuration file will be placed in the same directory as where the application's executable is located.");
                let curr_exe_path = std::env::current_exe();
                if curr_exe_path.is_err() {
                    eprintln!("Something went from fetching the executable's path. Exiting");
                    std::process::exit(1);
                }

                let curr_exe_path_unwrapped = curr_exe_path.unwrap();
                println!("You'll find your configuration file at {:?}", curr_exe_path_unwrapped.as_path());

                curr_exe_path_unwrapped
            }
        };

        //Check if the config file and folder exists
        if !config_path.as_path().exists() {
            //Convert the path to str and split it on the OS's path seperator
            let path_as_str = config_path.as_path().to_str().unwrap();
            let path_parts: Vec<&str> = path_as_str.split(std::path::MAIN_SEPARATOR).collect();

            //Construct a PathBuf containing the entire folder path, but not the file itself
            let mut folder_parts = PathBuf::new();
            let path_parts_for_folder = path_parts.len() -1;
            for i in 0..path_parts_for_folder {
                folder_parts.push(path_parts.get(i).unwrap());
            }

            //Create the configuration folder
            let dir_create_operation = std::fs::create_dir_all(folder_parts.as_path());
            if dir_create_operation.is_err() {
                println!("An error occurred while creating the configuration file directory: {:?}", dir_create_operation.err());
                std::process::exit(1);
            }

            //Example Configuration file content
            let example_config = Environment {
                mysql_host: "YOUR_MYSQL_HOST".to_string(),
                mysql_database: "YOUR MYSQL_DATABASE".to_string(),
                mysql_username: "YOUR MYSQL_USERNAME".to_string(),
                mysql_password: "YOUR_MYSQL_PASSWORD".to_string()
            };

            //Serialize to a String
            let example_config_as_str = serde_yaml::to_string(&example_config).unwrap();

            //Create the configuration file
            let config_file = std::fs::File::create(config_path.as_path());
            if config_file.is_err() {
                eprintln!("An error occurred while creating the configuration file: {:?}", config_file.err());
                std::process::exit(1);
            }

            //Write the example content to the configuration file
            let write_operation = config_file.unwrap().write_all(example_config_as_str.as_bytes());
            if write_operation.is_err() {
                eprintln!("An error occurred while creating the configuration file:{:?}", write_operation.is_err());
                std::process::exit(1)
            }

            //We're going to exit now to let the user configure the server.
            println!("An example configuration file has been created at {}, please configure it before restarting the application.", config_path.as_path().to_str().unwrap());
            std::process::exit(0);
        }

        //Read the configuration file to a String
        let config_file_content = std::fs::read_to_string(config_path.as_path());
        if config_file_content.is_err() {
            eprintln!("Unable to read configuration file: {:?}", config_file_content.err());
            std::process::exit(1);
        }

        //Deserialize the configuration file content
        let environment: serde_yaml::Result<Environment> = serde_yaml::from_str(&config_file_content.unwrap());
        if environment.is_err() {
            eprintln!("Something went wrong deserializing the configuration file content: {:?}", environment.err());
            std::process::exit(1);
        }

        environment.unwrap().clone()
    }

    fn get_environment_from_vars() -> Environment {
        use std::env::var;

        let mysql_host = var("MYSQL_HOST");
        if mysql_host.is_err() {
            Self::env_variable_not_set("MYSQL_HOST");
        }

        let mysql_database = var("MYSQL_DATABASE");
        if mysql_database.is_err() {
            Self::env_variable_not_set("MYSQL_DATABASE");
        }

        let mysql_username = var("MYSQL_USERNAME");
        if mysql_username.is_err() {
            Self::env_variable_not_set("MYSQL_USERNAME");
        }

        let mysql_password = var("MYSQL_PASSWORD");
        if mysql_password.is_err() {
            Self::env_variable_not_set("MYSQL_PASSWORD");
        }

        Environment {
            mysql_host:         mysql_host.unwrap(),
            mysql_database:     mysql_database.unwrap(),
            mysql_username:     mysql_username.unwrap(),
            mysql_password:     mysql_password.unwrap()
        }
    }

    fn env_variable_not_set(name: &str) {
        eprintln!("Required environmental variable '{}' not set. Exiting", name);
    }
}

impl Database {
    pub fn new(environment: &Environment) -> Database {
        //Construct the MySQL URL
        let mysql_uri = format!("mysql://{username}:{password}@{host}/{database}",
            username =  environment.mysql_username,
            password =  environment.mysql_password,
            host =      environment.mysql_host,
            database =  environment.mysql_database
        );

        //Create the Pool
        let pool = mysql::Pool::new(mysql_uri);
        if pool.is_err() {
            eprintln!("Unable to establish connection to the database: {:?}", pool.err());
            std::process::exit(1);
        }

        Database {
            pool: pool.unwrap()
        }
    }
}