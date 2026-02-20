# Sample Application

## how to run

```bash
cd examples/app
docker compose up --build
```

## tree

```
app/
   db/
      migrations/
         001_init.sql
   src/
      adapters.rs
      main.rs
   .env
   Cargo.toml
   Dockerfile
   README.md
```

## expected output

```
state-engine-app       | === state-engine Sample App ===
state-engine-app       | 
state-engine-app       | 1. Loading manifests from: ./manifest
state-engine-app       |    - Manifests loaded
state-engine-app       | 
state-engine-app       | 2. Setting up adapters...
state-engine-app       |    - InMemory adapters initialized
state-engine-app       |    - Env adapter initialized
state-engine-app       |    - KVS adapters initialized
state-engine-app       |    - DB adapter initialized
state-engine-app       | 
state-engine-app       | 3. Configuring Load module...
state-engine-app       |    - Load module configured
state-engine-app       | 
state-engine-app       | 4. Creating State...
state-engine-app       |    - State initialized
state-engine-app       | 
state-engine-app       | 5. Demo: Loading connection config from Env...
state-engine-app       |    Connection config loaded:
state-engine-app       |    {
state-engine-app       |   "charset": "UTF8",
state-engine-app       |   "database": "state_engine_dev",
state-engine-app       |   "driver": "postgres",
state-engine-app       |   "host": "postgres",
state-engine-app       |   "password": "state_pass",
state-engine-app       |   "port": "5432",
state-engine-app       |   "username": "state_user"
state-engine-app       | }
state-engine-app       | 
state-engine-app       | 6. Demo: Accessing nested values...
state-engine-app       |    connection.common.host: "postgres"
state-engine-app       | 
state-engine-app       | 7. Demo: State::exists()...
state-engine-app       |    connection.common.host exists: true
state-engine-app       | 
state-engine-app       | 8. Demo: Get metadata...
state-engine-app       |    _load metadata:
state-engine-app       |    {
state-engine-app       |   "client": "Env",
state-engine-app       |   "map": {
state-engine-app       |     "connection.common.database": "DB_DATABASE",
state-engine-app       |     "connection.common.host": "DB_HOST",
state-engine-app       |     "connection.common.password": "DB_PASSWORD",
state-engine-app       |     "connection.common.port": "DB_PORT",
state-engine-app       |     "connection.common.username": "DB_USERNAME"
state-engine-app       |   }
state-engine-app       | }
state-engine-app       | 
state-engine-app       | === Sample completed ===
state-engine-app exited with code 0
```
