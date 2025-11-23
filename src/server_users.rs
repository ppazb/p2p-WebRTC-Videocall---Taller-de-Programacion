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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServerUser {
    username: String,
    password: String,
    status: UserStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum UserStatus {
    Conectado,
    Desconectado,
    Ocupado,
}

///Recibo un path a un Json y a partir de eso genero mi UserLoginData
impl ServerUserData {
    pub fn new(file_path: &str) -> Result<Self, LoginErrors> {
        // Leo archivo
        let lectura = fs::read_to_string(file_path).map_err(|_| LoginErrors::FileNotFound)?;

        // Parseo Json a un vector de usuaruios
        let lista_usuarios: Vec<ServerUser> =
            serde_json::from_str(&lectura).map_err(|_| LoginErrors::FileIncompatible)?;

        // Genero un hashmap con los usarios donde la clave es el nonmbre de usuario
        let mut users = HashMap::new();
        for user in lista_usuarios {
            // TODO : checkearfsi clone es necesario
            users.insert(user.username.clone(), user);
        }

        Ok(Self {
            file_path: file_path.to_string(),
            users,
        })
    }

    ///devuelve una lista de todos los usuarios registrados en forma de string
    pub fn get_listado_usuarios(&self) -> Vec<String> {
        self.users.keys().cloned().collect()
    }

    /// dado un usuario y contraseña, verifico si son correctos
    pub fn verificar_login(&self, username: &str, password: &str) -> Result<(), LoginErrors> {
        let user = self.users.get(username).ok_or(LoginErrors::UserNotFound)?;

        if user.password != password {
            return Err(LoginErrors::IncorrectPassword);
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
            return Err(LoginErrors::OneUserMultipleLogins);
        }

        user.status = UserStatus::Conectado;
        Ok(())
    }

    /// a partir de un nombre de usuario, cambio su estado a desconectado
    pub fn desconectar(&mut self, username: &str) -> Result<(), LoginErrors> {
        let user = self
            .users
            .get_mut(username)
            .ok_or(LoginErrors::UserNotFound)?;

        user.status = UserStatus::Desconectado;
        Ok(())
    }

    /// a partir de un nombred de usuario, cambio su estado a ocupado
    pub fn ocupado(&mut self, username: &str) -> Result<(), LoginErrors> {
        let user = self
            .users
            .get_mut(username)
            .ok_or(LoginErrors::UserNotFound)?;

        user.status = UserStatus::Ocupado;
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
        Ok(())
    }

    pub fn add_user(&mut self, username: &str, password: &str) -> Result<(), LoginErrors> {
        // Si usuario ya existe, error
        if self.users.contains_key(username) {
            return Err(LoginErrors::UsernameAlreadyExists);
        }

        // Cre0 el usuario
        let user = ServerUser {
            username: username.to_string(),
            password: password.to_string(),
            status: UserStatus::Desconectado,
        };

        //Lo inserto en el HashMap
        self.users.insert(username.to_string(), user);

        // Guardar en el archivo
        self.guardar_cambios()?;

        Ok(())
    }
}
