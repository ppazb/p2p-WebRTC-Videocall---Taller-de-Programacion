# Taller de Programación I - Grupo Sobrevivir Taller

**RoomRTC** es una aplicación de videollamadas Peer-to-Peer (P2P) escrita en Rust sobre el stack de protocolos de WebRTC. Permite registrar usuarios e iniciar sesión, ver quién está conectado y llamar a otro usuario. Durante la llamada se transmiten **video y audio** en tiempo real y se pueden **enviar archivos** a la otra persona.

El trabajo principal fue implementar la pila de protocolos de WebRTC: algunos protocolos desde cero y otros con ayuda de crates.

## Funcionalidades

### Entrega de cursada

- **SDP (Session Description Protocol)**: generación y parseo de ofertas y respuestas para negociar la sesión entre pares.
- **ICE (Interactive Connectivity Establishment)**: recolección de candidatos, connectivity checks y selección del par de candidatos por el que viaja el media.
- **STUN**: obtención de candidatos *server reflexive* usando un servidor público (`stun.l.google.com:19302`).
- **Signalling Server con TLS**: servidor central con conexión TCP persistente cifrada con TLS (`rustls`). Maneja registro, login, estados de usuario (disponible / ocupado / desconectado), llamadas y el intercambio de SDP.
- **RTP / RTCP**: envío del video entre pares, con estadísticas de transmisión (sender/receiver reports) visibles en la UI.
- **DTLS + SRTP**: handshake DTLS (`openssl`) para inicializar el cifrado del media.
- **Video**: captura de cámara (`opencv`) y codificación/decodificación H264 (`openh264`).
- **Usuarios**: registro de usuarios nuevos con contraseña, inicio de sesión y listado de usuarios conectados con su estado actualizado en tiempo real.
- **Interfaz gráfica** (`eframe`/`egui`): vistas de Inicio de Sesión, Registro, Sala de Espera, Llamada entrante y Sala de Llamada.
- **Archivos de configuración**: cada componente se configura con un archivo JSON que se pasa por línea de comandos.
- **Logs**: registro de las acciones del cliente y del servidor.

### Agregado del examen final

- **Transmisión de audio**: captura y reproducción con `cpal`, codificación G.711 (A-law, 8 kHz) y envío por RTP junto al video. Botones para **apagar/encender el micrófono** y la cámara durante la llamada.
- **Transferencia de archivos por Data Channel**: envío de archivos de cualquier tipo y tamaño sobre SCTP encapsulado en DTLS. El destinatario puede **aceptar o rechazar** la descarga, y los archivos aceptados se guardan en disco.

## Transferencia de archivos

### Flujo

1. Durante la llamada, el usuario presiona **"Enviar archivo"** e ingresa la ruta del archivo. La UI valida que la ruta exista y sea un archivo.
2. El cliente envía `file_sending_request` al Signalling Server, que se lo reenvía al otro usuario como `file_sending_request_forwarded`.
3. Al destinatario le aparece una ventana para **aceptar o rechazar** el archivo:
   - Si **rechaza**, se envía `file_sending_reject` y al remitente le llega la notificación `file_sending_rejected`.
   - Si **acepta**, se envía `file_sending_accept` y al remitente le llega `file_sending_accepted`. Ahí empieza el envío por el Data Channel.
4. El archivo viaja **peer to peer** (no pasa por el servidor) por una asociación SCTP (`sctp-proto`) que corre sobre el mismo socket UDP cifrado con DTLS. Un demultiplexor (`MuxedUdpStream`) separa los paquetes DTLS de los RTP/RTCP.
5. El receptor guarda el archivo en `downloads/recibido_<nombre_original>`.

### Protocolo del Data Channel

La asociación SCTP se arma al empezar la llamada: quien llama toma el rol de cliente SCTP y quien recibe la llamada toma el rol de servidor. Cada archivo usa su **propio stream SCTP**, así que se pueden mandar varios archivos y en ambas direcciones.

Por cada archivo se envía:

1. **Header** (PPID `Binary`):

   | Campo           | Tamaño     | Descripción                         |
   |-----------------|------------|-------------------------------------|
   | `filename_len`  | 2 bytes    | Largo del nombre (u16, big endian)  |
   | `filename`      | variable   | Nombre del archivo en UTF-8         |
   | `file_size`     | 8 bytes    | Tamaño total (u64, big endian)      |

2. **Contenido** (PPID `BinaryEmpty`): el archivo se lee y se envía en chunks de hasta 10 KiB.

Del lado receptor, el primer mensaje de un stream nuevo se interpreta como header y crea el archivo de destino. Los mensajes siguientes se escriben en ese archivo hasta llegar a `file_size` bytes, y ahí se da la transferencia por terminada.

## UI

### Inicio de Sesión
<img width="1847" height="1023" alt="image" src="https://github.com/user-attachments/assets/bb8acaa7-9682-4a7d-8369-89565de29fbd" />

### Creación de nuevos usuarios

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/293522ad-91ca-4c6c-be6c-3d3507cc17b6" />

### Inicio de Sesión con usuario y contraseña
<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/6a775393-b133-4171-81a1-77437f898d3f" />

### Sala de Espera
<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/f68e851c-f360-4224-977e-ee5980619d00" />

### Visualización de la lista de usuarios conectados

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/c62c5839-c536-49bf-8642-d6b7058e438a" />

### Llamada

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/21ed253e-866c-4fe1-8292-152bb7d8b871" />
<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/5a8e3a32-ede3-49eb-be88-79e2fc7719de" />

### En Llamada

En la sala de llamada están los botones para apagar/encender la cámara y el micrófono, enviar un archivo y cortar la llamada, junto con las estadísticas de transmisión.

<img width="1854" height="1048" alt="image" src="https://github.com/user-attachments/assets/203ea7f6-a216-46bc-a48e-7a7be94c4f28" />

<!-- TODO: agregar capturas de "Enviar archivo", del pedido de aceptar/rechazar y de la notificación de rechazo -->

## Integrantes

- Rivero, Tobías 106302
- Vargas, Alejandro 104121
- Paz Blanco, Pilar 105600
- Chen, Michelle Melisa 105506

## Cómo usar

### Requisitos

- Linux con Rust estable.
- Dependencias del sistema (Ubuntu/Debian):

```
sudo apt-get install -y llvm-dev libclang-dev clang libopencv-dev pkg-config libasound2-dev libssl-dev
```

- Cámara y micrófono disponibles.

### Ejecución

Levantar el Signalling Server:

```
cargo run --bin signalling_server <path_config_server>
```

Levantar un cliente (uno por terminal o por computadora):

```
cargo run --bin room_rtc <path_config_cliente>
```

Por ejemplo, con los archivos de ejemplo del repositorio:

```
cargo run --bin signalling_server src/server_config.json
cargo run --bin room_rtc src/prueba_config.json
```

Los certificados TLS que usa el servidor están en `certs/`.

### Archivo de configuración del servidor

```json
{
    "server_adder": "192.168.0.15:8000",
    "users_db": "src/users.json"
}
```

- `server_adder`: IP y puerto donde escucha el Signalling Server.
- `users_db`: archivo donde se guardan los usuarios registrados.

### Archivo de configuración del cliente

```json
{
    "general_info": {
        "server_adder": "192.168.0.15"
    },

    "sdp_handshake": {
        "local_adder": "192.168.0.114:8090",
        "server_adder": "192.168.0.15:8000",
        "codecs": ["H264"],
        "ice_candidates": ["192.168.0.114:8500", "192.168.0.114:8250"],
        "offer_path": "src/offer.sdp"
    }
}
```

- `local_adder`: dirección local del cliente.
- `server_adder` (en `sdp_handshake`): IP y puerto del Signalling Server.
- `codecs`: códecs de video a ofrecer en el SDP.
- `ice_candidates`: direcciones locales usadas como candidatos *host* de ICE.
- `offer_path`: archivo donde se escribe la oferta SDP.

### Salidas

- Archivos recibidos: `downloads/recibido_<nombre_original>`.
- Logs: `src/logs/log_server.txt` y `src/logs/log_client.txt`.

### Tests y lint

```
cargo test
cargo clippy --all-targets --all-features
cargo fmt --check
```
