# Challenge
Caché distribuido y Arquitectura de Servidor de Juegos

## Soluciones
Los documentos de cada solución se encuentran en /soluciones

## Ejecutar código
Para esto necesitaremos rust en la versión 1.89.0
NOTA: puedes crear los .env apartir de los .env example, o ejecutar de la siguiente manera:

### Tests
Algunos test se realizaron usando el standard de Rust, sin embargo, para mayor legibilidad los de los Use Cases y Servicios se realizaron en la carpeta dentro de la apps/{app_name}/src/tests
```sh
cargo test
```

### Iniciar Master Node
```sh
PORT=5555 cargo run -p cache_master
```

### Iniciar Master Cache Node
```sh
MASTER_IPS="127.0.0.1:5555" ROLE="MASTER" cargo run -p cache_node
```

### Iniciar Replica Node
```sh
MASTER_IPS="127.0.0.1:5555" ROLE="REPLICA" cargo run -p cache_node
```

### Iniciar Cliente
```sh
CACHE_IPS="127.0.0.1:5555" cargo run -p cache_client
```
