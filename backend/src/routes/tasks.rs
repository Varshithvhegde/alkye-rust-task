
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Extension,
    Json,
    Router,
};

use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    middleware::auth::AuthUser,
    models::task::TaskView,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: String,
}

#[derive(Debug, Deserialize)]
pub struct AssignTaskRequest {
    pub task_id: String,
    pub assigned_to_id: String,
}

#[derive(Debug, FromRow)]
struct StaffUser {
    id: String,
    email: String,
    role: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_task))
        .route("/assign", post(assign_task))
        .route("/view-my-tasks", get(view_my_tasks))
}

// --------------------------------------------------
// POST /tasks
// Admin only
// --------------------------------------------------

async fn create_task(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&user)?;

    let title = payload.title.trim();

    if title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Task title cannot be empty".to_string(),
        ));
    }

    if !["low", "medium", "high"]
        .contains(&payload.priority.as_str())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Priority must be low, medium, or high".to_string(),
        ));
    }

    let task_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO tasks
         (id, title, description, status, priority,
          created_by_id, assigned_to_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(&task_id)
    .bind(title)
    .bind(&payload.description)
    .bind("todo")
    .bind(&payload.priority)
    .bind(&user.id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    let task = sqlx::query_as::<_, TaskView>(
        "SELECT
            t.id,
            t.title,
            t.description,
            t.status,
            t.priority,
            t.created_by_id,
            t.assigned_to_id,
            u.email AS assigned_to,
            t.created_at,
            t.updated_at
         FROM tasks t
         LEFT JOIN users u
            ON u.id = t.assigned_to_id
         WHERE t.id = ?",
    )
    .bind(&task_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    Ok(Json(json!({
        "message": "Task created successfully",
        "task": task
    })))
}

// --------------------------------------------------
// POST /tasks/assign
// Admin only
// --------------------------------------------------

async fn assign_task(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<AssignTaskRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    require_admin(&user)?;

    // Ensure the assignee exists and is a staff user.
    let staff = sqlx::query_as::<_, StaffUser>(
        "SELECT id, email, role
         FROM users
         WHERE id = ?",
    )
    .bind(&payload.assigned_to_id)
    .fetch_optional(&state.db)
    .await
    .map_err(internal_error)?;

    let Some(staff) = staff else {
        return Err((
            StatusCode::NOT_FOUND,
            "Assigned user not found".to_string(),
        ));
    };

    if staff.role != "staff" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Tasks can only be assigned to staff users".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();

    let result = sqlx::query(
        "UPDATE tasks
         SET assigned_to_id = ?,
             updated_at = ?
         WHERE id = ?",
    )
    .bind(&staff.id)
    .bind(&now)
    .bind(&payload.task_id)
    .execute(&state.db)
    .await
    .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Task not found".to_string(),
        ));
    }

    // Invalidate the assigned user's cached task list.
    state.task_cache.invalidate(&staff.id).await;

    let task = sqlx::query_as::<_, TaskView>(
        "SELECT
            t.id,
            t.title,
            t.description,
            t.status,
            t.priority,
            t.created_by_id,
            t.assigned_to_id,
            u.email AS assigned_to,
            t.created_at,
            t.updated_at
         FROM tasks t
         LEFT JOIN users u
            ON u.id = t.assigned_to_id
         WHERE t.id = ?",
    )
    .bind(&payload.task_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal_error)?;

    Ok(Json(json!({
        "message": "Task assigned successfully",
        "task": task
    })))
}

// --------------------------------------------------
// GET /tasks/view-my-tasks
// Authenticated user
// --------------------------------------------------

async fn view_my_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Check per-user cache first.
    if let Some(tasks) = state.task_cache.get(&user.id).await {
        return Ok(Json(json!({
            "user": {
                "email": user.email,
                "role": user.role
            },
            "tasks": tasks,
            "summary": {
                "total_assigned_tasks": tasks.len()
            },
            "cache": {
                "hit": true
            }
        })));
    }

    // Cache miss: load tasks from SQLite.
    let tasks = sqlx::query_as::<_, TaskView>(
        "SELECT
            t.id,
            t.title,
            t.description,
            t.status,
            t.priority,
            t.created_by_id,
            t.assigned_to_id,
            u.email AS assigned_to,
            t.created_at,
            t.updated_at
         FROM tasks t
         LEFT JOIN users u
            ON u.id = t.assigned_to_id
         WHERE t.assigned_to_id = ?
         ORDER BY t.created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await
    .map_err(internal_error)?;

    let total = tasks.len();

    // Store result in cache for this user.
    state
        .task_cache
        .insert(user.id.clone(), tasks.clone())
        .await;

    Ok(Json(json!({
        "user": {
            "email": user.email,
            "role": user.role
        },
        "tasks": tasks,
        "summary": {
            "total_assigned_tasks": total
        },
        "cache": {
            "hit": false
        }
    })))
}

// --------------------------------------------------
// Helpers
// --------------------------------------------------

fn require_admin(
    user: &AuthUser,
) -> Result<(), (StatusCode, String)> {
    if user.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            "Only Admin can perform this action".to_string(),
        ));
    }

    Ok(())
}

fn internal_error<E: std::fmt::Display>(
    error: E,
) -> (StatusCode, String) {
    tracing::error!("Task API error: {}", error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}
