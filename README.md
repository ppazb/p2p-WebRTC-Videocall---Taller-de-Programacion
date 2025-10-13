# Taller de Programacion {Grupo}

## Integrantes

## Como usar 

A continuacion se detallan los pasos para compilar y ejecutar el programa.

### Compilacion

### Como correr
Abrir 2 terminales, luego en la primera ejecutar:
export OPENCV_VIDEOIO_PRIORITY_MSMF=0
export OPENCV_VIDEOIO_PRIORITY_GSTREAMER=0
export OPENCV_VIDEOIO_PRIORITY_V4L2=100
cargo run --release --bin client 127.0.0.1:4000 127.0.0.1:5000 src/offer2.sd

Luego en la segunda terminar:
cargo run --release --bin client_receptor 127.0.0.1:5000 127.0.0.1:4000 src/offer.sdp

## Como testear