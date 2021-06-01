// EndBASIC
// Copyright 2021 Julio Merino
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License.  You may obtain a copy
// of the License at:
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.  See the
// License for the specific language governing permissions and limitations
// under the License.

//! Commands to interact with the cloud service.

use crate::console::{read_line, Console};
use crate::service::*;
use crate::storage::Storage;
use async_trait::async_trait;
use endbasic_core::ast::{ArgSep, Expr, Value, VarType};
use endbasic_core::exec::Machine;
use endbasic_core::syms::{
    CallError, CallableMetadata, CallableMetadataBuilder, Command, CommandResult,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str;

/// Category description for all symbols provided by this module.
const CATEGORY: &str = "Cloud access
The EndBASIC service, should you choose to create an account in, is a cloud service that provides \
online file sharing across users of EndBASIC.  The commands below allow you interact with this \
service once you have created an account.
During account creation time, you are assigned a unique, persistent drive in which you can store \
files privately.  You can later choose to share individual files with the public or with specific \
individuals, at which point those people will be able to see them by mounting your drive.
Once logged in, the cloud:// file system scheme becomes available.  You can use it to mount other \
people's drives by specifying their username as the path.  For example, a command like the \
following would mount user-123's shared files as a new drive X: MOUNT \"X\", \"cloud://user-123\"";

/// The `LOGIN` command.
pub struct LoginCommand {
    metadata: CallableMetadata,
    service: Rc<RefCell<dyn Service>>,
    console: Rc<RefCell<dyn Console>>,
    storage: Rc<RefCell<Storage>>,
}

impl LoginCommand {
    /// Creates a new `LOGIN` command.
    pub fn new(
        service: Rc<RefCell<dyn Service>>,
        console: Rc<RefCell<dyn Console>>,
        storage: Rc<RefCell<Storage>>,
    ) -> Rc<Self> {
        Rc::from(Self {
            metadata: CallableMetadataBuilder::new("LOGIN", VarType::Void)
                .with_syntax("username$ [password$]")
                .with_category(CATEGORY)
                .with_description(
                    "Logs into the user's account.
Please note that the username$ provided here is actually the email address that you used during \
the sign up process.
On a successful login, this mounts your personal drive under the CLOUD:/ location, which you can \
access with any other file-related commands.  Using the cloud:// file system scheme, you can mount \
other people's drives with the MOUNT command.",
                )
                .build(),
            service,
            console,
            storage,
        })
    }

    /// Performs the login workflow against the server.
    async fn do_login(&self, username: &str, password: &str) -> CommandResult {
        let access_token = self.service.borrow_mut().authenticate(&username, &password).await?;

        let username;
        let mut first = true;
        let mut data = HashMap::new();
        loop {
            let request = LoginRequest { data: data.clone() };
            let result = self.service.borrow_mut().login(&access_token, &request).await;
            match result {
                Ok(Ok(response)) => {
                    let console = &mut *self.console.borrow_mut();
                    if !response.motd.is_empty() {
                        // TODO(jmmv): This should probably use the code in the help module to
                        // refill paragraphs, but we ought to generalize that first.
                        console.print("")?;
                        console.print("----- BEGIN SERVER MOTD -----")?;
                        for line in response.motd {
                            console.print(&line)?;
                        }
                        console.print("-----  END SERVER MOTD  -----")?;
                        console.print("")?;
                    }
                    username = response.username;
                    break;
                }
                Ok(Err(response)) => {
                    let missing_data = match response.missing_data {
                        Some(md) if !md.is_empty() => md,
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Server denied login but did not tell us why",
                            )
                            .into())
                        }
                    };

                    let console = &mut *self.console.borrow_mut();
                    if first {
                        console.print("")?;
                        console.print("It looks like this is the first time you log in.")?;
                        console.print(
                            "We need some extra information to finish setting up your account.",
                        )?;
                        first = false;
                    } else {
                        console.print("")?;
                        console.print(&format!("Server error: {}", response.message))?;
                    }
                    console.print("")?;

                    data.clear();
                    for field in missing_data {
                        let value = read_line(console, &format!("{}? ", field), "").await?;
                        data.insert(field, value);
                    }
                }
                Err(e) => return Err(e.into()),
            };
        }

        let mut storage = self.storage.borrow_mut();
        storage.register_scheme(
            "cloud",
            Box::from(CloudDriveFactory::new(self.service.clone(), access_token)),
        );
        storage.mount("CLOUD", &format!("cloud://{}", username))?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl Command for LoginCommand {
    fn metadata(&self) -> &CallableMetadata {
        &self.metadata
    }

    async fn exec(&self, args: &[(Option<Expr>, ArgSep)], machine: &mut Machine) -> CommandResult {
        if self.storage.borrow().has_scheme("cloud") {
            // TODO(jmmv): To support authenticating more than once in one session, we have to
            // either refresh the access tokens of any mounted drive or unmount them all.  Plus we
            // have to avoid re-registering or re-creating the "cloud" scheme.
            return Err(CallError::InternalError(
                "Support for calling LOGIN twice in the same session is not implemented".to_owned(),
            ));
        }

        let (username, password) = match args {
            [(Some(username), ArgSep::End)] => match username.eval(machine.get_mut_symbols())? {
                Value::Text(username) => {
                    // TODO(jmmv): Implement a safe version of read_line that does not echo input.
                    let password =
                        read_line(&mut *self.console.borrow_mut(), "Password: ", "").await?;
                    (username, password)
                }
                _ => {
                    return Err(CallError::ArgumentError(
                        "LOGIN requires a string as the username".to_owned(),
                    ))
                }
            },
            [(Some(username), ArgSep::Long), (Some(password), ArgSep::End)] => {
                let username = match username.eval(machine.get_mut_symbols())? {
                    Value::Text(username) => username,
                    _ => {
                        return Err(CallError::ArgumentError(
                            "LOGIN requires a string as the username".to_owned(),
                        ))
                    }
                };
                let password = match password.eval(machine.get_mut_symbols())? {
                    Value::Text(password) => password,
                    _ => {
                        return Err(CallError::ArgumentError(
                            "LOGIN requires a string as the password".to_owned(),
                        ))
                    }
                };
                (username, password)
            }
            _ => {
                return Err(CallError::ArgumentError(
                    "LOGIN requires one or two arguments".to_owned(),
                ))
            }
        };

        match self.do_login(&username, &password).await {
            Ok(()) => Ok(()),
            e => {
                if !username.contains('@') {
                    self.console
                        .borrow_mut()
                        .print("Login failed; remember that usernames should be email addresses")?;
                }
                e
            }
        }
    }
}

/// Adds all remote manipulation commands for `service` to the `machine`, using `console` to
/// display information and `storage` to manipulate the remote drives.
pub(crate) fn add_all(
    machine: &mut Machine,
    service: Rc<RefCell<dyn Service>>,
    console: Rc<RefCell<dyn Console>>,
    storage: Rc<RefCell<Storage>>,
) {
    machine.add_command(LoginCommand::new(service, console, storage));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutils::*;

    #[test]
    fn test_login_ok_with_password() {
        let mut t = Tester::default();
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data: HashMap::default() },
            Ok(Ok(LoginResponse { username: "yo".to_owned(), motd: vec![] })),
        );
        assert!(!t.get_storage().borrow().mounted().contains_key("CLOUD"));
        t.run(format!(r#"LOGIN "{}", "{}""#, MockService::USERNAME, MockService::PASSWORD)).check();
        assert!(t.get_storage().borrow().mounted().contains_key("CLOUD"));
    }

    #[test]
    fn test_login_ok_ask_password() {
        let t = Tester::default();
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data: HashMap::default() },
            Ok(Ok(LoginResponse { username: "yo".to_owned(), motd: vec![] })),
        );
        let storage = t.get_storage();
        assert!(!storage.borrow().mounted().contains_key("CLOUD"));
        t.add_input_chars(MockService::PASSWORD)
            .add_input_chars("\n")
            .run(format!(r#"LOGIN "{}""#, MockService::USERNAME))
            .check();
        assert!(storage.borrow().mounted().contains_key("CLOUD"));
    }

    #[test]
    fn test_login_show_motd() {
        let mut t = Tester::default();
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data: HashMap::default() },
            Ok(Ok(LoginResponse {
                username: "yo".to_owned(),
                motd: vec!["first line".to_owned(), "second line".to_owned()],
            })),
        );
        t.run(format!(r#"LOGIN "{}", "{}""#, MockService::USERNAME, MockService::PASSWORD))
            .expect_prints([
                "",
                "----- BEGIN SERVER MOTD -----",
                "first line",
                "second line",
                "-----  END SERVER MOTD  -----",
                "",
            ])
            .check();
    }

    #[test]
    fn test_login_incomplete_account_fill_details() {
        let t = Tester::default();

        let data = HashMap::default();
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data },
            Ok(Err(ErrorResponse {
                message: "".to_owned(),
                missing_data: Some(vec!["field1".to_owned(), "field2".to_owned()]),
            })),
        );

        let mut data = HashMap::default();
        data.insert("field1".to_owned(), "field1 response".to_owned());
        data.insert("field2".to_owned(), "field2 response".to_owned());
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data },
            Ok(Err(ErrorResponse {
                message: "please retry".to_owned(),
                missing_data: Some(vec!["field1".to_owned(), "field3".to_owned()]),
            })),
        );

        let mut data = HashMap::default();
        data.insert("field1".to_owned(), "field1 second response".to_owned());
        data.insert("field3".to_owned(), "field3 response".to_owned());
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data },
            Ok(Ok(LoginResponse { username: "yo".to_owned(), motd: vec![] })),
        );

        let storage = t.get_storage();
        assert!(!storage.borrow().mounted().contains_key("CLOUD"));
        t.add_input_chars("field1 response\n")
            .add_input_chars("field2 response\n")
            .add_input_chars("field1 second response\n")
            .add_input_chars("field3 response\n")
            .run(format!(r#"LOGIN "{}", "{}""#, MockService::USERNAME, MockService::PASSWORD))
            .expect_prints([
                "",
                "It looks like this is the first time you log in.",
                "We need some extra information to finish setting up your account.",
                "",
                "",
                "Server error: please retry",
                "",
            ])
            .check();
        assert!(storage.borrow().mounted().contains_key("CLOUD"));
    }

    #[test]
    fn test_login_incomplete_account_invalid_reply() {
        for missing_data in vec![None, Some(vec![])].into_iter() {
            let mut t = Tester::default();

            let data = HashMap::default();
            t.get_service().borrow_mut().add_mock_login(
                LoginRequest { data },
                Ok(Err(ErrorResponse { message: "".to_owned(), missing_data })),
            );

            let storage = t.get_storage().clone();
            assert!(!storage.borrow().mounted().contains_key("CLOUD"));
            t.run(format!(r#"LOGIN "{}", "{}""#, MockService::USERNAME, MockService::PASSWORD))
                .expect_err("Server denied login but did not tell us why")
                .check();
            assert!(!storage.borrow().mounted().contains_key("CLOUD"));
        }
    }

    #[test]
    fn test_login_bad_credentials() {
        let mut t = Tester::default();
        t.run(format!(r#"LOGIN "{}", "{}""#, "bad-user@example.com", MockService::PASSWORD))
            .expect_err("Unknown user")
            .check();
        t.run(format!(r#"LOGIN "{}", "{}""#, MockService::USERNAME, "bad-password"))
            .expect_err("Invalid password")
            .check();
        assert!(!t.get_storage().borrow().mounted().contains_key("CLOUD"));
    }

    #[test]
    fn test_login_bad_credentials_username_is_not_email() {
        let mut t = Tester::default();
        t.run(format!(r#"LOGIN "{}", "{}""#, "bad-user", MockService::PASSWORD))
            .expect_err("Unknown user")
            .expect_prints(["Login failed; remember that usernames should be email addresses"])
            .check();
        assert!(!t.get_storage().borrow().mounted().contains_key("CLOUD"));
    }

    #[test]
    fn test_login_twice_not_supported() {
        let mut t = Tester::default();
        t.get_service().borrow_mut().add_mock_login(
            LoginRequest { data: HashMap::default() },
            Ok(Ok(LoginResponse { username: "yo".to_owned(), motd: vec![] })),
        );
        assert!(!t.get_storage().borrow().mounted().contains_key("CLOUD"));
        t.run(format!(
            r#"LOGIN "{}", "{}": LOGIN "a", "b""#,
            MockService::USERNAME,
            MockService::PASSWORD
        ))
        .expect_err("Support for calling LOGIN twice in the same session is not implemented")
        .check();
        assert!(t.get_storage().borrow().mounted().contains_key("CLOUD"));
    }

    #[test]
    fn test_login_errors() {
        check_stmt_err("LOGIN requires one or two arguments", r#"LOGIN"#);
        check_stmt_err("LOGIN requires one or two arguments", r#"LOGIN "a", "b", "c""#);
        check_stmt_err("LOGIN requires a string as the username", r#"LOGIN 3"#);
        check_stmt_err("LOGIN requires a string as the username", r#"LOGIN 3, "a""#);
        check_stmt_err("LOGIN requires a string as the password", r#"LOGIN "a", 3"#);
    }
}
