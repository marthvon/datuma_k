# Example: Event + Venue

FastAPI + Vite React generated from one dtct contract. Handwritten glue is the in-memory HTTP server, fetch client, and router. Generated code is types, validators, datetime transforms, and CRUD pages.

Commands below assume this directory (`tests/example`) is the current working directory so `.env` is loaded from this project, not the repo root.

## Generate

```bash
cargo run --manifest-path ../../Cargo.toml -- run
```

## Backend

Listens on http://localhost:8000.

```bash
cd backend
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn main:app --reload
```

## Frontend

In another terminal. Dev server is http://localhost:5173 (CORS is allowed from that origin).

```bash
cd frontend
npm install
npm run dev
```

Open http://localhost:5173. Create or update an event with capacity outside 1–500: Zod rejects it in the browser; FastAPI would reject the same payload if it got through.

`cargo test` regenerates this example and asserts the emitted output. It does not start uvicorn or npm.

