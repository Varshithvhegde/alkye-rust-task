# Task Management System

A full-stack task management application built with **Rust, Axum,
SQLite, and React + TypeScript**.

The application supports role-based task management: - **Admin** users
can create tasks and assign them to staff. - **Staff** users can view
tasks assigned to them. - Authentication uses JWTs. - Staff login
includes a two-factor verification step using a development OTP flow. -
Task responses include a cache status to demonstrate task-list
caching. - Assignment updates invalidate the affected staff member's
task cache.

> **Development status:** This project is being developed as a coding
> assignment. Check [LIMITATIONS.md](./LIMITATIONS.md) for known
> limitations and current issues.

## Tech Stack

### Backend

-   Rust
-   Axum
-   SQLite
-   JWT-based authentication
-   Moka in-memory cache
-   Tokio
-   Tower HTTP CORS

### Frontend

-   React
-   TypeScript
-   Vite

## Project Structure

``` text
rust-react-starter/
├── src/                    # Rust backend source
│   ├── main.rs
│   ├── routes/
│   │   ├── auth.rs
│   │   ├── tasks.rs
│   │   ├── users.rs
│   │   └── ...
│   ├── middleware/
│   └── state.rs
└── frontend/
    ├── src/
    │   ├── App.tsx
    │   └── style.css
    ├── package.json
    └── vite.config.ts
```

The exact modules may differ slightly depending on the current
implementation.

## Prerequisites

Install: - Rust toolchain (stable) - Cargo - Node.js and npm

## Run Locally

### 1. Start the backend

From the repository root:

``` bash
cargo run
```

The backend is configured to listen at:

``` text
http://127.0.0.1:8080
```

Make sure the SQLite database is initialized according to the project's
database setup/migrations.

### 2. Start the frontend

Open a second terminal:

``` bash
cd frontend
npm install
npm run dev
```

Vite normally serves the frontend at:

``` text
http://localhost:5173
```

The frontend API base URL should point to `http://localhost:8080`.

## Development Seed Accounts

The development seed data includes:

  Role    Email                      Password
  ------- -------------------------- -------------
  Admin   `admin@example.com`        `Admin@123`
  Staff   `james.bond@example.com`   `Bond@123`

These credentials are for local development only. Do not use them in
production.

## Main Workflow

1.  Sign in as the admin.
2.  Complete the development OTP verification flow.
3.  Create tasks.
4.  Assign tasks to a staff user.
5.  Sign in as the staff user.
6.  View tasks assigned to that user.
7.  Repeat the task-list request to observe cache behavior.

## API Overview

The routes below describe the intended API contract. Confirm the actual
route registration in `src/main.rs` and `src/routes/tasks.rs` before
relying on them.

  ------------------------------------------------------------------------------
  Method            Endpoint                 Purpose           Access
  ----------------- ------------------------ ----------------- -----------------
  `POST`            `/auth/...`              Login /           Public
                                             authentication    
                                             flow              

  `POST`            `/tasks`                 Create a task     Admin

  `POST`            `/tasks/assign`          Assign a task to  Admin
                                             staff             

  `GET`             `/tasks/view-my-tasks`   View tasks        Authenticated
                                             assigned to       staff
                                             current user      
  ------------------------------------------------------------------------------

The exact authentication endpoint names and request/response fields are
defined by the auth route implementation.

### Create a task

Intended endpoint:

``` http
POST /tasks
Authorization: Bearer <JWT>
Content-Type: application/json
```

Example body:

``` json
{
  "title": "Prepare weekly report",
  "description": "Summarize this week's progress",
  "priority": "medium"
}
```

### Assign a task

Intended endpoint:

``` http
POST /tasks/assign
Authorization: Bearer <JWT>
Content-Type: application/json
```

Example body:

``` json
{
  "task_id": 1,
  "assigned_to_id": 2
}
```

### View assigned tasks

Intended endpoint:

``` http
GET /tasks/view-my-tasks
Authorization: Bearer <JWT>
```

The response includes the user's tasks and cache information, such as
whether the response was served from cache.

## Authentication and Authorization

-   Protected task routes require a valid JWT.
-   Admin-only actions check the authenticated user's role.
-   Staff users should not be able to create or assign tasks.
-   A development OTP endpoint is available for local testing.
-   The development OTP mechanism is not intended for production use.

## Cache Behavior

The task-list endpoint uses an in-memory cache keyed by user ID.

Expected behavior: 1. First request after cache miss queries SQLite and
populates the cache. 2. A repeated request may return a cache hit. 3.
Assigning a task invalidates the relevant staff user's cached task list.

Cache is process-local and is cleared when the backend restarts.

## Troubleshooting

### `404 Not Found` for `/tasks`

Check that the backend route is registered with the same path the
frontend calls.

If task routes are merged directly into the main router, the task router
should define absolute paths such as:

``` rust
.route("/tasks", post(create_task))
.route("/tasks/assign", post(assign_task))
.route("/tasks/view-my-tasks", get(view_my_tasks))
```

If the task router defines `/`, `/assign`, and `/view-my-tasks`, mount
it using `.nest("/tasks", protected_task_routes)` instead.

Do not combine both approaches, or paths may be duplicated (for example
`/tasks/tasks`).

After changing routes: 1. Stop the backend process. 2. Run `cargo run`
again. 3. Check the backend terminal for compilation errors. 4. Inspect
the browser Network tab to confirm the exact URL and HTTP method.

### Frontend cannot reach backend

-   Confirm the backend is running on port `8080`.
-   Confirm the frontend API base URL is `http://localhost:8080`.
-   Confirm CORS allows `http://localhost:5173`.
-   Check browser DevTools Network and Console tabs.

### Unauthorized or forbidden responses

-   `401 Unauthorized`: missing, expired, or invalid JWT.
-   `403 Forbidden`: authenticated user lacks the required role.

## Security Notes

-   Development credentials and OTP utilities must be removed or secured
    before deployment.
-   Use environment variables or a secret manager for JWT and OTP
    secrets.
-   Use HTTPS in production.
-   Add request validation, rate limiting, and stronger session/token
    management before production use.
-   Never commit real credentials, tokens, or secrets.

## License

Add the license applicable to this project before publishing or
distributing it.
