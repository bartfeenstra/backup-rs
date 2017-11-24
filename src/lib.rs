#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;
pub mod errors {
    error_chain! { }
}
use errors::*;

extern crate chrono;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use chrono::offset::Utc;
use serde::de::{Deserialize, Deserializer, Visitor, MapAccess};
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::result::Result as StdResult;
use std::rc::Rc;
use std::ffi::OsString;

pub trait Target: fmt::Display {
    fn is_ready(&self, environment: Rc<Environment>) -> Result<()>;

    fn backup(&self, environment: Rc<Environment>) -> Result<()>;
}

#[derive(Debug, Deserialize)]
pub struct SshRsyncTarget {
    // The name of the user on the target.
    user: String,

    // The target host.
    host: String,

    // The network port. Usually 22.
    pub port: usize,

    // The path on the target to back up to. Must be owned by this application.
    path: String,
}

impl SshRsyncTarget {
    pub fn to_ssh(&self) -> String {
        format!("{}@{}:{}", self.user.as_str(), self.host.as_str(), self.path.as_str())
    }

    fn backup_create_snapshot(&self, environment: Rc<Environment>) -> Result<()> {
        let date = Utc::now().format("%Y-%m-%d_%H-%M-%S_UTC");
        environment.log(format!("{}", date).as_str());
        self.remote_command(environment, "[ -d \"backup-latest\" ] && cp -al `readlink backup-latest` backup-$date && rm backup-latest || [ ! -d \"backup-$date\" ] && mkdir backup-$date || ln -s backup-$date backup-latest")
    }

//    fn sync(&self) -> Result<()> {
//        // rsync -ar --delete --delete-excluded --include-from=$current_directory/backup-files --numeric-ids --progress -e "ssh -i $ssh_key" --verbose -v $source_path $target_user@$target_host:$target_path/backup-latest
//        //        println!(">> rsync {} {}", self.configuration.source_path, self.configuration.target_path);
//        let options = vec![
//            "-r",
//            "-v",
//            "--exclude=.[a-zA-Z0-9]*",
//            "--filter=:- .gitignore",
//            "--delete"];
//        // @todo Don't actually sync anything anywhere just yet. Can we write a safe integration
//        // test instead?
//        //        self.command("rsync", options)?;
//        // @todo Message errors too.
//        self.message("Back-up complete.")
//    }

    fn remote_command(&self, environment: Rc<Environment>, command: &str) -> Result<()> {
        // @todo We need to take the remote executable and options, and escape them properly for use in an argument.
        let ssh_target = self.to_ssh();
        let options = vec!["-i", "~/.ssh/id_rsa", ssh_target.as_str(), command];
        environment.command("ssh", options)
    }
}

impl Target for SshRsyncTarget {
    fn is_ready(&self, environment: Rc<Environment>) -> Result<()> {
        environment.log(format!("Trying to connect to target {}.", self).as_str());
        self.remote_command(environment, "exit")
    }

    fn backup(&self, environment: Rc<Environment>) -> Result<()> {
        environment.message("Hi there!")?;
        //        Ok(())
        //
        //
        //
        //  so long store [story] short Rc<Sring> would be like a &String, but with no lifetime to fanagle with
        //
        //
        //
        self.backup_create_snapshot(environment)?;
        Ok(())
        //        self.sync()
    }
}

impl fmt::Display for SshRsyncTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}:{}, port {}", self.user.as_str(), self.host.as_str(), self.path.as_str(), self.port)
    }
}

pub struct Environment {
    pub configuration: Configuration,
}

impl Environment {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration: configuration,
        }
    }

    pub fn log(&self, message: &str) {
        if self.configuration.verbose {
            println!("{}", message);
        }
    }

    pub fn message(&self, message: &str) -> Result<()> {
        self.log(message);
        if let Some(ref user) = self.configuration.notify_user {
            let options = vec!["-i", "-u", user.as_str(), "notify-send", message];
            return self.command("sudo", options)
        }
        Ok(())
    }

    pub fn command<I, S>(&self, executable: &str, arguments: I) -> Result<()>
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
    {
        let mut command = Command::new(executable);
        let command = command.args(arguments.into_iter())
            .stdin(Stdio::null())
            .stderr(Stdio::piped());
        let stdout = match self.configuration.verbose {
            true => Stdio::piped(),
            false => Stdio::null(),
        };
        let command = command.stdout(stdout);
        let output = command.output()
            .chain_err(||format!("Failed to execute: {}.", executable))?;
        // @todo These should be piped?
        if output.stdout.len() > 0 {
            self.log(format!("stdout: {}", String::from_utf8_lossy(&output.stdout)).as_str());
        }
        if output.stderr.len() > 0 {
            self.log(format!("stderr: {}", String::from_utf8_lossy(&output.stderr)).as_str());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Configuration {
    // Whether or not to output detailed information.
    pub verbose: bool,

    // The local user to send status updates to.
    pub notify_user: Option<String>,

    // The absolute path to the local data to back up.
    pub source_path: String,

    // The targets to try.
    pub targets: Vec<Rc<SshRsyncTarget>>,
}

impl Configuration {
    pub fn from_file(file_path: &Path) -> Result<Self> {
        let mut file = File::open(file_path).chain_err(||"Error opening configuration file.")?;
        let mut configuration_data = String::new();
        file.read_to_string(&mut configuration_data).chain_err(|| "Error reading configuration file.")?;
        Configuration::from_toml(configuration_data)
    }

    fn from_toml(toml: String) -> Result<Self> {
        let configuration: Self = toml::from_str(toml.as_ref()).chain_err(|| "Error parsing configuration file as TOML.")?;
        Ok(configuration)
    }
}

pub struct Runner {
    environment: Rc<Environment>,
    _target_index: Option<usize>,
}

impl Runner {
    pub fn new(environment: Environment) -> Self {
        Self {
            environment: Rc::new(environment),
            _target_index: None,
        }
    }

    pub fn backup(&mut self) -> Result<()> {
        self.target()?.backup(self.environment.clone())
    }

    fn target(&mut self) -> Result<Rc<Target>> {
        if let Some(index) = self._target_index {
            return Ok(self.environment.configuration.targets[index].clone())
        }

        for (index, target) in self.environment.configuration.targets.iter().enumerate() {
            let environment = self.environment.clone();
            environment.log(format!("Checking if target {} is ready...", target).as_str());
            match target.clone().is_ready(environment) {
                Err(e) => {
                    self.environment.log(format!("Target {} is not ready: {}.", target, e).as_str());
                    continue
                },
                _ => {
                    self._target_index = Some(index);
                    self.environment.log(format!("Using target {}.", target).as_str());
                    return Ok(target.clone())
                }
            }
        }
        Err("Could not connect to any of the targets.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_config_should_succeed_if_valid() {
        let configuration_file_path = "./tests/resources/configuration/backup.toml";
        let configuration = Configuration::from_file(Path::new(&configuration_file_path)).unwrap();
        assert_eq!(configuration.notify_user.unwrap(), "bart");
    }

    #[test]
    #[should_panic]
    fn parse_toml_config_should_fail_if_invalid() {
        let configuration_file_path = "./tests/resources/configuration/backup-invalid.toml";
        Configuration::from_file(Path::new(&configuration_file_path)).unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_toml_config_should_fail_if_incomplete() {
        let configuration_file_path = "./tests/resources/configuration/backup-incomplete.toml";
        Configuration::from_file(Path::new(&configuration_file_path)).unwrap();
    }
}
