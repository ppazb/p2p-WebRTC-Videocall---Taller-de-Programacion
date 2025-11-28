# Taller de Programacion - Grupo Sobrevivir Taller

En este presente trabajo desarrollamos una versión del Stack WebRTC en el lenguaje Rust, de manera tal que sea posible la realización de videoconferencia entre pares de usuarios. 

Los componentes son: 

- SDP (Session Description Protocol)
- ICE (Interactive Connectivity Establishment)
- Servidor STUN
- Comunicación encriptada con el Signalling Server (TLS)
- Transmisión de Video (Encodeo y Desencodeo)
- SRTP (Secure Real-time Transport Protocol)
- DTLS (Datagram Transport Layer Security)
- Interfaz UI de la Aplicación

## Integrantes

- Rivero, Tobías 106302
- Vargas, Alejandro 104121
- Paz Blanco, Pilar 105600
- Chen, Michelle Melisa 105506


## Como usar 

A continuacion se detallan los pasos para compilar y ejecutar el programa. 

Para levantar el Signalling Server: 

```
cargo run --bin signalling_server  path_config_file
```

Para levantar el Client en una terminal: 

```
cargo run --bin room_rtc  path_config_file
```
## TLS Certificates y Keys 

### Creación de Certificates y Keys
```
openssl x509 -in cert.pem -outform DER -out cert.der                 
openssl pkcs8 -topk8 -inform PEM -outform DER -in key.pem -out key.der -nocrypt

openssl req \
  -x509 \           
  -newkey rsa:2048 \                                                           
  -keyout key.pem \
  -out cert.pem \
  -days 365 \
  -nodes \
  -subj "/CN=localhost" \
  -addext "basicConstraints=CA:FALSE" \
  -addext "keyUsage=digitalSignature, keyEncipherment" \
  -addext "extendedKeyUsage=serverAuth" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"


```

