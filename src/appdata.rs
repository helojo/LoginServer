use std::path::PathBuf;
use std::io::Write;
use serde::{Deserialize, Serialize};
use rand::Rng;
use mysql::prelude::Queryable;
use mysql::{Row, Params, params};
use std::collections::HashMap;

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
    pub mysql_password: String,

    pub password_pepper:  String
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
            //Convert the path to str and split it on the OS's path separator
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
                mysql_password: "YOUR_MYSQL_PASSWORD".to_string(),
                password_pepper: rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(64).map(char::from).collect()
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

        let password_pepper = var("PASSWORD_PEPPER");
        if password_pepper.is_err() {
            Self::env_variable_not_set("PASSWORD_PEPPER");
        }

        Environment {
            mysql_host:         mysql_host.unwrap(),
            mysql_database:     mysql_database.unwrap(),
            mysql_username:     mysql_username.unwrap(),
            mysql_password:     mysql_password.unwrap(),
            password_pepper:    password_pepper.unwrap()
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

    pub fn check_db(&self, environment: &Environment) -> Result<bool, ()> {
        let conn_wrapped = self.pool.get_conn();
        if conn_wrapped.is_err() {
            eprintln!("An error occurred: {:?}", conn_wrapped.err().unwrap());
            return Err(());
        }
        let mut conn = conn_wrapped.unwrap();

        let sql_fetch_tables_wrapped = conn.exec::<Row, &str, Params>("SELECT table_name FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = :table_schema", params! {
            "table_schema" => environment.mysql_database.clone()
        });

        if sql_fetch_tables_wrapped.is_err() {
            eprintln!("An error occurred: {:?}", sql_fetch_tables_wrapped.unwrap());
            return Err(());
        }

        let sql_fetch_tables = sql_fetch_tables_wrapped.unwrap();
        let mut required_tables_map = HashMap::new();
        required_tables_map.insert("users".to_string(), false);
        required_tables_map.insert("sessions".to_string(), false);

        for row in sql_fetch_tables {
            let table_name = row.get::<String, &str>("table_name").unwrap();
            required_tables_map.insert(table_name.clone(), true);
        }

        let mut db_passed = true;
        for entry in required_tables_map.iter() {
            if *entry.1 == false {
                eprintln!("Missing table: '{}'", entry.0);
                db_passed = false;
            }
        }
        Ok(db_passed)
    }

    pub fn init_db(&self, environment: &Environment) -> Result<(), ()> {
        //Create a connection
        let conn_wrapped = self.pool.get_conn();
        if conn_wrapped.is_err() {
            eprintln!("An error occurred: {:?}", conn_wrapped.err().unwrap());
            return Err(());
        }
        let mut conn = conn_wrapped.unwrap();

        //Create 'sessions' table
        let sql_create_sessions_table = conn.query::<usize, &str>(format!("CREATE TABLE `{}`.`sessions` ( `session_id` VARCHAR(64) NOT NULL , `user_id` VARCHAR(64) NOT NULL , `expiry` BIGINT NOT NULL , PRIMARY KEY (`session_id`)) ENGINE = InnoDB;", environment.mysql_database.clone()).as_str());

        if sql_create_sessions_table.is_err() {
            eprintln!("An error occurred: {:?}", sql_create_sessions_table.err().unwrap());
            return Err(());
        }
        println!("Created table 'sessions'");

        //Create 'users' table
        let sql_create_users_table = conn.query::<usize, &str>(format!("CREATE TABLE `{}`.`users` ( `user_id` VARCHAR(64) NOT NULL , `email` VARCHAR(255) NOT NULL , `password` VARCHAR(255) NOT NULL , `salt` VARCHAR(16) NOT NULL , PRIMARY KEY (`user_id`)) ENGINE = InnoDB;", environment.mysql_database.clone()).as_str());

        if sql_create_users_table.is_err() {
            eprintln!("An error occurred: {:?}", sql_create_users_table.err().unwrap());
            return Err(());
        }
        println!("Created table 'users'.");

        Ok(())
    }
}