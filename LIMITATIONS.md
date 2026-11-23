# Limitations and Known Issues

This document records the current limitations of the task management
assignment. It should be updated as the implementation changes.

## Known Issue

### Task endpoint may return `404 Not Found`

The frontend has been observed requesting:

``` text
POST http://localhost:8080/tasks
```

A `404` indicates that the requested method/path combination is not
registered in the running backend (or the frontend is reaching a
different backend instance). It is not normally caused by JWT
authentication; authentication failures should generally return `401` or
`403`.

Check route registration consistency:

**Option A --- absolute paths with router merge**

In `routes/tasks.rs`:

``` rust
Router::new()
    .route("/tasks", post(create_task))
    .route("/tasks/assign", post(assign_task))
    .route("/tasks/view-my-tasks", get(view_my_tasks))
```

In `main.rs`, merge the protected task router:

``` rust
.merge(protected_task_routes)
```

**Option B --- relative paths with router nest**

In `routes/tasks.rs`:

``` rust
Router::new()
    .route("/", post(create_task))
    .route("/assign", post(assign_task))
    .route("/view-my-tasks", get(view_my_tasks))
```

In `main.rs`, mount it:

``` rust
.nest("/tasks", protected_task_routes)
```

Use one option only. Do not define `/tasks/...` and also nest the router
at `/tasks`.

If the 404 remains: - Stop and restart the backend process after code
changes. - Confirm the terminal is running the intended project and
binary. - Confirm the frontend request method is `POST` and the URL is
exactly `/tasks`. - Check whether the route is `/tasks/` and whether the
client/backend normalize trailing slashes. - Check for another service
or stale backend running on port `8080`. - Review the complete router
assembly and any additional `nest`, `merge`, or fallback handlers.

## Functional Limitations

-   The application is intended for a single admin/staff workflow and
    does not currently describe a full organization/team management
    system.
-   The development OTP flow is not a production-grade second-factor
    provider.
-   Seeded credentials are fixed development credentials and must not be
    used in a deployed environment.
-   The OTP retrieval endpoint or in-memory OTP storage is
    development-only and should not be exposed publicly.
-   Task cache is in-memory and process-local. It does not persist
    across restarts and is not shared between multiple backend
    instances.
-   Cache invalidation is implemented around task assignment; any
    additional task updates or deletes must also invalidate the relevant
    cache.
-   The current task priority model is limited to the supported values
    (`low`, `medium`, `high`) if enforced by the backend.
-   API validation and error handling may need additional coverage for
    malformed JSON, missing fields, invalid IDs, and database failures.
-   CORS is configured for local development (`localhost:5173`) and
    needs a production frontend origin before deployment.

## Production Readiness Gaps

Before production deployment, consider: - Move all secrets and database
configuration into environment variables. - Replace development OTP
delivery/retrieval with a real verified channel. - Add rate limiting for
login and OTP endpoints. - Add password reset and account recovery
flows. - Add structured logging and request tracing. - Add database
migrations and backup/restore procedures. - Add pagination and filtering
for task lists. - Add tests for authentication, role checks, task
creation, assignment, and cache invalidation. - Configure production
CORS, HTTPS, and secure token handling. - Add a proper deployment
configuration and health-check endpoint.

## Scope

This document distinguishes observed issues from general hardening work.
Items under **Known Issue** were reported during local development;
other items are potential limitations or production-readiness
improvements and should be verified against the current code.
