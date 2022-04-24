# Sample - Products API

## Usage
To run you require to have an instance of redis running. For that you can
use the `docker-compose.yml` file to run the redis container.

```bash
docker compose up -d
```

## Endpoints
Base URL: http://localhost:8080

You can change the port in the code, or setting the environment variable `PORT`.

Method | Endpoint
--- | ---
GET | `/products`
GET | `/products/{id}`
POST | `/products`
PUT | `/products/{id}`
DELETE | `/products/{id}`