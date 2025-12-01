use crate::server_users_errors::LoginErrors;
use std::collections::HashMap;
use std::fs;

#[derive(Debug)]
///A partir de un path traigo a memoria los usarios del server. Se toma como identificador unico su nombre de usuario y eso trae todos sus datos
/// aplico struct User para facilitar manejo de datos
pub struct ServerUserData {
    file_path: String,
    users: HashMap<String, ServerUser>,
}

/// Struct para representar el usuario en el servidor, contiene datos que perduran como datos volatiles como el estado de conexion
#[derive(Debug, Clone)]
pub struct ServerUser {
    username: String,
    password: String,
    status: UserStatus,
}

/// Struct utilizado de forma temporal para parsear los datos perdurables de los usuarios
#[derive(Debug, Clone)]
struct UserLoginData {
    username: String,
    password: String,
}

///Estado de conexion del usuario relevantes al login
#[derive(Debug, Clone, PartialEq)]
pub enum UserStatus {
    Conectado,
    Desconectado,
}

///Recibo un path a un Json y a partir de eso genero mi UserLoginData
impl ServerUserData {
    pub fn new(file_path: &str) -> Result<Self, LoginErrors> {
        // Leo archivo
        let lectura = fs::read_to_string(file_path).map_err(|_| LoginErrors::FileNotFound)?;

        // Parseo Json a un vector de usuaruios
        let lista_usuarios = parse_json_login(&lectura)?;

        // Genero un hashmap con los usarios donde la clave es el nonmbre de usuario
        let mut users = HashMap::new();
        for user in lista_usuarios {
            users.insert(
                user.username.clone(),
                ServerUser {
                    username: user.username,
                    password: user.password,
                    status: UserStatus::Desconectado,
                },
            );
        }

        eprintln!(
            "[SERVER USERS] Loaded {} users from {}",
            users.len(),
            file_path
        );
        Ok(Self {
            file_path: file_path.to_string(),
            users,
        })
    }

    ///devuelve una lista de todos los usuarios registrados en forma de string
    pub fn get_listado_usuarios(&self) -> Vec<String> {
        eprintln!(
            "[SERVER USERS] Returning list of {} users",
            self.users.len()
        );
        self.users.keys().cloned().collect()
    }

    /// dado un usuario y contraseña, verifico si son correctos
    pub fn verificar_login(&self, username: &str, password: &str) -> Result<(), LoginErrors> {
        let user = self.users.get(username).ok_or(LoginErrors::UserNotFound)?;

        if user.password != password {
            eprintln!(
                "[SERVER USERS] failed login for '{}': incorrect password",
                username
            );
            return Err(LoginErrors::IncorrectPassword);
        }

        //si esta ya conectado error
        if user.status == UserStatus::Conectado {
            eprintln!(
                "[SERVER USERS] User '{}' failed to login: Multiple logins not allowed",
                username
            );
            return Err(LoginErrors::OneUserMultipleLogins);
        }

        eprintln!("[SERVER USERS] successful login for '{}'", username);
        Ok(())
    }

    /// Valida las credenciales ingresadas para registrar un nuevo usuario.
    ///
    /// Se definen condiciones de aceptación para el nuevo username y el password, como longitudes y caracteres permitidos.
    /// Si alguna condición no se cumple, o si el usuario ya existe, se devuelve el correspondiente LoginError.
    pub fn verificar_signup(&self, username: &str, password: &str) -> Result<(), LoginErrors> {
        // Si usuario ya existe, error
        if self.users.contains_key(username) {
            return Err(LoginErrors::UsernameAlreadyExists);
        }

        // Valido condiciones del usuario (5 a 32 caracteres)
        if username.len() < 5 || username.len() > 32 {
            return Err(LoginErrors::InvalidFieldsOnSignUp); // Longitud usuario inválida
        }

        // Valido que los dígitos sean solo alfanuméricos
        for c in username.chars() {
            if !c.is_ascii_alphanumeric() {
                return Err(LoginErrors::InvalidFieldsOnSignUp); // Caracteres inválidos
            }
        }

        // Valido longitud de contraseña
        if password.len() < 5 || username.len() > 32 {
            return Err(LoginErrors::InvalidFieldsOnSignUp); // Longitud contraseña inválida
        }

        Ok(())
    }

    /// a partir de un nombre de usuario, cambio su estado a conectado
    pub fn conectar(&mut self, username: &str) -> Result<(), LoginErrors> {
        let user = self
            .users
            .get_mut(username)
            .ok_or(LoginErrors::UserNotFound)?;

        //si el usuario ya esta conectado, error
        if user.status == UserStatus::Conectado {
            eprintln!(
                "[SERVER USERS] User '{}' failed to login: Multiple logins not allowed",
                username
            );
            return Err(LoginErrors::OneUserMultipleLogins);
        }

        user.status = UserStatus::Conectado;
        eprintln!("[SERVER USERS] User '{}' connected", username);
        Ok(())
    }

    /// a partir de un nombre de usuario, cambio su estado a desconectado
    pub fn desconectar(&mut self, username: &str) -> Result<(), LoginErrors> {
        let user = self
            .users
            .get_mut(username)
            .ok_or(LoginErrors::UserNotFound)?;

        user.status = UserStatus::Desconectado;
        eprintln!("[SERVER USERS] User '{}' disconnected", username);
        Ok(())
    }

    /// guardo los cambios en el archivo Json
    pub fn guardar_cambios(&self) -> Result<(), LoginErrors> {
        let users: Vec<&ServerUser> = self.users.values().collect();

        let mut json = String::from("[\n");
        for (i, user) in users.iter().enumerate() {
            json.push_str("  {\n");
            json.push_str(&format!("\"username\": \"{}\",\n", user.username));
            json.push_str(&format!("\"password\": \"{}\"\n", user.password));
            json.push_str("  }");

            if i < users.len() - 1 {
                json.push_str(",");
            }
            json.push_str("\n");
        }

        json.push_str("]");

        // lo guardamos
        fs::write(&self.file_path, json).map_err(|_| LoginErrors::FileNotFound)?;
        eprintln!(
            "[SERVER USERS] Changes saved to {}, {} total users",
            self.file_path,
            self.users.len()
        );
        Ok(())
    }

    pub fn add_user(&mut self, username: &str, password: &str) -> Result<(), LoginErrors> {
        // Si usuario ya existe, error
        if self.users.contains_key(username) {
            eprintln!(
                "[SERVER USERS] Failed to add user '{}': username already exists",
                username
            );
            return Err(LoginErrors::UsernameAlreadyExists);
        }

        // Creo el usuario
        let user = ServerUser {
            username: username.to_string(),
            password: password.to_string(),
            status: UserStatus::Desconectado,
        };

        //Lo inserto en el HashMap
        self.users.insert(username.to_string(), user);

        // Guardar en el archivo
        self.guardar_cambios()?;
        eprintln!("[SERVER USERS] User '{}' added", username);

        Ok(())
    }
}

impl ServerUser {
    pub fn get_username(&self) -> String {
        self.username.clone()
    }
}

fn parse_json_login(json_str: &str) -> Result<Vec<UserLoginData>, LoginErrors> {
    let mut users = Vec::new();
    let str_json = json_str.trim();

    // manejo de caso vacio
    if str_json == "[]" {
        return Ok(users);
    }

    // saco []
    if !str_json.starts_with('[') || !str_json.ends_with(']') {
        return Err(LoginErrors::FileIncompatible);
    }

    let inner_content = &str_json[1..str_json.len() - 1].trim();
    // otro manejo de caso vacio
    if inner_content.is_empty() {
        return Ok(users);
    }

    let items: Vec<&str> = inner_content.split("},").map(|s| s.trim()).collect();
    for item in items {
        let mut item_trim = item.trim();

        // Saco {}
        if item_trim.starts_with('{') {
            item_trim = &item_trim[1..];
        }
        if item_trim.ends_with('}') {
            item_trim = &item_trim[..item_trim.len() - 1];
        }

        let mut username = None;
        let mut password = None;

        // Separo key - valor
        let keys_values: Vec<&str> = item_trim.split(',').map(|s| s.trim()).collect();

        for key_value in keys_values {
            let kv: Vec<&str> = key_value
                .split(':')
                .map(|s| s.trim().trim_matches('"'))
                .collect();

            if kv.len() != 2 {
                continue;
            }

            match kv[0] {
                "username" => username = Some(kv[1].to_string()),
                "password" => password = Some(kv[1].to_string()),
                _ => {}
            }
        }

        if let (Some(username), Some(password)) = (username, password) {
            users.push(UserLoginData { username, password });
        } else {
            return Err(LoginErrors::FileIncompatible);
        }
    }

    Ok(users)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    /// Trabajo con archivos temporales para no generar archivos de mas
    /// Crea un archivo temporal con el contenido dado y devuelve el NamedTempFile
    fn create_test_file(content: &str) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), content).unwrap();
        file
    }

    #[test]
    /// Prueba que la función new carga correctamente los usuarios desde un archivo JSON
    fn test_new_loads_users_correctly() {
        let test_data = r#"
        [
            {"username": "user1", "password": "pass1"},
            {"username": "user2", "password": "pass2"}
        ]
        "#;

        let file = create_test_file(test_data);
        let server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        assert_eq!(server_data.users.len(), 2);
        assert!(server_data.users.contains_key("user1"));
        assert!(server_data.users.contains_key("user2"));
    }

    #[test]
    /// Prueba que la funcion verificar_login retorna Ok para credenciales correctas
    fn test_verificar_login_success() {
        let test_data = r#"
        [
            {"username": "testuser", "password": "testpass"}
        ]
        "#;

        let file = create_test_file(test_data);
        let server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        assert!(server_data.verificar_login("testuser", "testpass").is_ok());
    }

    #[test]
    /// Prueba que la funcion verificar_login retorna un error para una contraseña incorrecta
    fn test_verificar_login_wrong_password() {
        let test_data = r#"
        [
            {"username": "testuser", "password": "testpass"}
        ]
        "#;

        let file = create_test_file(test_data);
        let server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        let result = server_data.verificar_login("testuser", "wrongpass");
        assert!(result.is_err());
    }

    #[test]
    /// Prueba que la funcion conectar y desconectar funcionan correctamente
    fn test_conectar_and_desconectar() {
        let test_data = r#"
        [
            {"username": "testuser", "password": "testpass"}
        ]
        "#;

        let file = create_test_file(test_data);
        let mut server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        // conecto
        assert!(server_data.conectar("testuser").is_ok());
        assert_eq!(
            server_data.users.get("testuser").unwrap().status,
            UserStatus::Conectado
        );

        // desconecto
        assert!(server_data.desconectar("testuser").is_ok());
        assert_eq!(
            server_data.users.get("testuser").unwrap().status,
            UserStatus::Desconectado
        );
    }

    #[test]
    /// Prueba que la funcion add_user agrega un nuevo usuario correctamente y no permite duplicados
    fn test_add_user() {
        let file = create_test_file("[]");
        let mut server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        // agrego nuevo usuario
        assert!(server_data.add_user("newuser", "newpass").is_ok());
        assert!(server_data.users.contains_key("newuser"));
        assert!(server_data.verificar_login("newuser", "newpass").is_ok());

        // Usuario duplicado debería dar error
        let result = server_data.add_user("newuser", "otherpass");
        assert!(result.is_err());
    }

    #[test]
    /// Prueba que la funcion get_listado_usuarios retorna la lista correcta de usuarios
    fn test_get_listado_usuarios() {
        let test_data = r#"
        [
            {"username": "user1", "password": "pass1"},
            {"username": "user2", "password": "pass2"}
        ]
        "#;

        let file = create_test_file(test_data);
        let server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        let usuarios = server_data.get_listado_usuarios();
        assert_eq!(usuarios.len(), 2);
        assert!(usuarios.contains(&"user1".to_string()));
        assert!(usuarios.contains(&"user2".to_string()));
    }

    #[test]
    /// Prueba que la funcion verificar_login retorna un error cuando el usuario no existe
    fn test_verificar_login_user_not_found() {
        let test_data = r#"
        [
            {"username": "testuser", "password": "testpass"}
        ]
        "#;

        let file = create_test_file(test_data);
        let server_data = ServerUserData::new(file.path().to_str().unwrap()).unwrap();

        let result = server_data.verificar_login("nonexistent", "password");
        assert!(result.is_err());
    }
}
