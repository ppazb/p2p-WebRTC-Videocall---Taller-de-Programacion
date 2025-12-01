use crate::server_users_errors::LoginErrors;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerUser {
    username: String,
    password: String,
    status: UserStatus,
}

/// Struct utilizado de forma temporal para parsear los datos perdurables de los usuarios
#[derive(Debug, Deserialize, Serialize, Clone)]
struct UserLoginData {
    username: String,
    password: String,
}

///Estado de conexion del usuario relevantes al login
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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
        let lista_usuarios: Vec<UserLoginData> =
            serde_json::from_str(&lectura).map_err(|_| LoginErrors::FileIncompatible)?;

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
        //No necesito las claves del hashmap, solo los valores
        let lista_usuarios: Vec<&ServerUser> = self.users.values().collect();

        //serializo
        let json = serde_json::to_string_pretty(&lista_usuarios)
            .map_err(|_| LoginErrors::FileIncompatible)?;

        //escribo en el archivo
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
