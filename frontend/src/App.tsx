
import { useEffect, useState } from "react";
import type { FormEvent } from "react";

// src/App.tsx
import "./style.css";


const API_URL =
  import.meta.env.VITE_API_URL || "http://localhost:8080";

// ==================== TYPES ====================

type UserRole = "admin" | "staff";

type User = {
  id: string;
  email: string;
  role: UserRole;
};

type StaffUser = {
  id: string;
  email: string;
  role: "staff";
};

type TaskStatus = "todo" | "in_progress" | "done";

type TaskPriority = "low" | "medium" | "high";

type Task = {
  id: string;
  title: string;
  description?: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  created_by_id?: string;
  assigned_to_id?: string | null;
  assigned_to?: string | null;
  created_at?: string;
  updated_at?: string;
};

type ApiError = Error & {
  status?: number;
};

type LoginResponse = {
  challenge_id?: string;
  login_challenge_id?: string;
  message?: string;
};

type VerifyResponse = {
  token?: string;
  access_token?: string;
  message?: string;
};

type CreateTaskResponse = {
  task?: Task;
  message?: string;
};

type AssignTaskResponse = {
  message?: string;
};

type MyTasksResponse = {
  tasks?: Task[];
  cache?: {
    hit?: boolean;
  };
  message?: string;
};

type DevOtpResponse = {
  email?: string;
  code?: string;
};

// ==================== API HELPER ====================

async function apiRequest<T>(
  path: string,
  options: RequestInit & { token?: string } = {}
): Promise<T> {
  const { token, ...requestOptions } = options;

  const response = await fetch(`${API_URL}${path}`, {
    ...requestOptions,
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...requestOptions.headers,
    },
  });

  const data = await response.json().catch(() => ({}));

  if (!response.ok) {
    const error = new Error(
      data.message ||
        data.error ||
        `Request failed (${response.status})`
    ) as ApiError;

    error.status = response.status;
    throw error;
  }

  return data as T;
}

// ==================== JWT HELPER ====================

function decodeJwt(token: string): User | null {
  try {
    const payload = token.split(".")[1];

    if (!payload) {
      return null;
    }

    const decoded = JSON.parse(
      atob(payload.replace(/-/g, "+").replace(/_/g, "/"))
    );

    if (
      !decoded.sub ||
      !decoded.email ||
      !decoded.role
    ) {
      return null;
    }

    return {
      id: decoded.sub,
      email: decoded.email,
      role: decoded.role as UserRole,
    };
  } catch {
    return null;
  }
}

// ==================== MAIN APP ====================

export default function App() {
  // Authentication
  const [token, setToken] = useState(
    () => localStorage.getItem("task_token") || ""
  );

  const [user, setUser] = useState<User | null>(() => {
    const savedToken = localStorage.getItem("task_token");

    return savedToken ? decodeJwt(savedToken) : null;
  });

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [challengeId, setChallengeId] = useState("");
  const [otp, setOtp] = useState("");

  // UI
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");

  // Admin tasks
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] =
    useState<TaskPriority>("medium");

  // Staff selection and assignment
  const [staffUsers, setStaffUsers] = useState<StaffUser[]>([]);
  const [selectedStaffId, setSelectedStaffId] = useState("");
  const [selectedTaskId, setSelectedTaskId] = useState("");

  // Staff task dashboard
  const [myTasks, setMyTasks] = useState<Task[]>([]);
  const [cacheHit, setCacheHit] = useState<boolean | null>(null);

  // Load appropriate dashboard data after login
  useEffect(() => {
    if (!token || !user) {
      return;
    }

    if (user.role === "admin") {
      void loadStaffUsers(token);
    }

    if (user.role === "staff") {
      void loadMyTasks(token);
    }
  }, [token, user?.role]);

  // ==================== MESSAGES ====================

  function showError(message: string) {
    setError(message);
    setSuccess("");
  }

  function showSuccess(message: string) {
    setSuccess(message);
    setError("");
  }

  // ==================== LOGIN ====================

  async function handleLogin(
    event: FormEvent<HTMLFormElement>
  ) {
    event.preventDefault();

    setLoading(true);
    setError("");
    setSuccess("");

    try {
      const result = await apiRequest<LoginResponse>(
        "/auth/login",
        {
          method: "POST",
          body: JSON.stringify({
            email,
            password,
          }),
        }
      );

      const receivedChallengeId =
        result.challenge_id ||
        result.login_challenge_id;

      if (!receivedChallengeId) {
        throw new Error(
          "Backend did not return a challenge ID."
        );
      }

      setChallengeId(receivedChallengeId);

      showSuccess(
        "Login successful. Please complete 2FA."
      );
    } catch (err) {
      showError(
        err instanceof Error
          ? err.message
          : "Login failed."
      );
    } finally {
      setLoading(false);
    }
  }

  // ==================== DEVELOPMENT OTP ====================

  async function getDevelopmentOtp() {
    setLoading(true);
    setError("");
    setSuccess("");

    try {
      const result = await apiRequest<DevOtpResponse>(
        "/dev/email-logs/latest"
      );

      if (!result.code) {
        throw new Error(
          "No OTP found in development email logs."
        );
      }

      setOtp(result.code);

      showSuccess(
        `Development OTP retrieved for ${result.email || email}.`
      );
    } catch (err) {
      showError(
        err instanceof Error
          ? err.message
          : "Could not retrieve OTP."
      );
    } finally {
      setLoading(false);
    }
  }

  // ==================== VERIFY OTP ====================

  async function handleVerifyOtp(
    event: FormEvent<HTMLFormElement>
  ) {
    event.preventDefault();

    setLoading(true);
    setError("");
    setSuccess("");

    try {
      const result = await apiRequest<VerifyResponse>(
        "/auth/verify-2fa",
        {
          method: "POST",
          body: JSON.stringify({
            login_challenge_id: challengeId,
            code: otp,
          }),
        }
      );

      const receivedToken =
        result.token || result.access_token;

      if (!receivedToken) {
        throw new Error(
          "Backend did not return an authentication token."
        );
      }

      const decodedUser = decodeJwt(receivedToken);

      if (!decodedUser) {
        throw new Error(
          "Unable to decode user information from JWT."
        );
      }

      localStorage.setItem("task_token", receivedToken);

      setToken(receivedToken);
      setUser(decodedUser);

      setChallengeId("");
      setOtp("");

      showSuccess("Authentication successful.");
    } catch (err) {
      showError(
        err instanceof Error
          ? err.message
          : "OTP verification failed."
      );
    } finally {
      setLoading(false);
    }
  }

  // ==================== LOGOUT ====================

  function logout() {
    localStorage.removeItem("task_token");

    setToken("");
    setUser(null);

    setChallengeId("");
    setOtp("");

    setTasks([]);
    setMyTasks([]);
    setStaffUsers([]);

    setSelectedStaffId("");
    setSelectedTaskId("");

    setCacheHit(null);

    setError("");
    setSuccess("");
  }

  // ==================== LOAD STAFF USERS ====================

  async function loadStaffUsers(
    currentToken: string = token
  ) {
    setLoading(true);
    setError("");

    try {
      const result = await apiRequest<User[]>(
        "/users",
        {
          token: currentToken,
        }
      );

      const staff = result.filter(
        (item): item is StaffUser =>
          item.role === "staff"
      );

      setStaffUsers(staff);

      // Keep the selected user if still available.
      setSelectedStaffId((currentId) => {
        const stillExists = staff.some(
          (item) => item.id === currentId
        );

        if (stillExists) {
          return currentId;
        }

        return staff[0]?.id || "";
      });
    } catch (err) {
      showError(
        err instanceof Error
          ? err.message
          : "Could not load staff users."
      );
    } finally {
      setLoading(false);
    }
  }

  // ==================== CREATE TASK ====================

  async function createTask(
    event: FormEvent<HTMLFormElement>
  ) {
    event.preventDefault();

    setLoading(true);
    setError("");
    setSuccess("");

    try {
      const result = await apiRequest<CreateTaskResponse>(
        "/tasks",
        {
          method: "POST",
          token,
          body: JSON.stringify({
            title: title.trim(),
            description: description.trim() || null,
            priority,
          }),
        }
      );

      if (result.task) {
        setTasks((previous) => [
          result.task!,
          ...previous,
        ]);

        setSelectedTaskId(result.task.id);
      }

      setTitle("");
      setDescription("");
      setPriority("medium");

      showSuccess(
        result.message || "Task created successfully."
      );
    } catch (err) {
      const apiError = err as ApiError;

      if (apiError.status === 403) {
        showError(
          "Forbidden: Staff users cannot create tasks."
        );
      } else {
        showError(
          err instanceof Error
            ? err.message
            : "Could not create task."
        );
      }
    } finally {
      setLoading(false);
    }
  }

  // ==================== ASSIGN TASK ====================

  async function assignTask(
    event: FormEvent<HTMLFormElement>
  ) {
    event.preventDefault();

    if (!selectedTaskId || !selectedStaffId) {
      showError("Please select a task and staff member.");
      return;
    }

    setLoading(true);
    setError("");
    setSuccess("");

    try {
      const result = await apiRequest<AssignTaskResponse>(
        "/tasks/assign",
        {
          method: "POST",
          token,
          body: JSON.stringify({
            task_id: selectedTaskId,
            assigned_to_id: selectedStaffId,
          }),
        }
      );

      const selectedStaff = staffUsers.find(
        (staff) => staff.id === selectedStaffId
      );

      showSuccess(
        result.message ||
          `Task assigned to ${selectedStaff?.email || "staff member"}.`
      );
    } catch (err) {
      const apiError = err as ApiError;

      if (apiError.status === 403) {
        showError(
          "Forbidden: Only Admin can assign tasks."
        );
      } else {
        showError(
          err instanceof Error
            ? err.message
            : "Could not assign task."
        );
      }
    } finally {
      setLoading(false);
    }
  }

  // ==================== STAFF TASKS ====================

  async function loadMyTasks(
    currentToken: string = token
  ) {
    setLoading(true);
    setError("");

    try {
      const result = await apiRequest<MyTasksResponse>(
        "/tasks/view-my-tasks",
        {
          token: currentToken,
        }
      );

      setMyTasks(result.tasks || []);
      setCacheHit(result.cache?.hit ?? null);
    } catch (err) {
      showError(
        err instanceof Error
          ? err.message
          : "Could not load your tasks."
      );
    } finally {
      setLoading(false);
    }
  }

  // ==================== LOGIN VIEW ====================

  if (!token || !user) {
    return (
      <div className="app-shell">
        <div className="auth-card">
          <div className="brand">
            <div className="brand-icon">T</div>

            <div>
              <h1>TaskFlow</h1>
              <p>Task Management System</p>
            </div>
          </div>

          {!challengeId ? (
            <form onSubmit={handleLogin}>
              <h2>Welcome back</h2>

              <p className="muted">
                Sign in to continue to your workspace.
              </p>

              <label>Email address</label>

              <input
                type="email"
                value={email}
                onChange={(event) =>
                  setEmail(event.target.value)
                }
                placeholder="admin@example.com"
                autoComplete="username"
                required
              />

              <label>Password</label>

              <input
                type="password"
                value={password}
                onChange={(event) =>
                  setPassword(event.target.value)
                }
                placeholder="Enter your password"
                autoComplete="current-password"
                required
              />

              <button disabled={loading}>
                {loading ? "Signing in..." : "Sign in"}
              </button>

              <div className="demo-credentials">
                <strong>Demo accounts</strong>

                <p>Admin: admin@example.com</p>
                <p>Password: Admin@123</p>

                <p>Staff: james.bond@example.com</p>
                <p>Password: Bond@123</p>
              </div>
            </form>
          ) : (
            <form onSubmit={handleVerifyOtp}>
              <h2>Two-factor authentication</h2>

              <p className="muted">
                Enter the 6-digit verification code.
              </p>

              <label>OTP code</label>

              <input
                value={otp}
                onChange={(event) =>
                  setOtp(
                    event.target.value.replace(/\D/g, "")
                  )
                }
                placeholder="123456"
                maxLength={6}
                inputMode="numeric"
                autoComplete="one-time-code"
                required
              />

              <button disabled={loading || otp.length !== 6}>
                {loading ? "Verifying..." : "Verify OTP"}
              </button>

              <button
                type="button"
                className="secondary"
                onClick={getDevelopmentOtp}
                disabled={loading}
              >
                Get development OTP
              </button>

              <button
                type="button"
                className="text-button"
                onClick={() => {
                  setChallengeId("");
                  setOtp("");
                  setError("");
                  setSuccess("");
                }}
              >
                Back to login
              </button>
            </form>
          )}

          <Message error={error} success={success} />
        </div>
      </div>
    );
  }

  // ==================== DASHBOARD ====================

  return (
    <div className="dashboard">
      <header className="topbar">
        <div className="brand">
          <div className="brand-icon">T</div>

          <div>
            <h1>TaskFlow</h1>
            <p>Task Management</p>
          </div>
        </div>

        <div className="user-section">
          <div className="user-info">
            <strong>{user.email}</strong>

            <span className={`role ${user.role}`}>
              {user.role}
            </span>
          </div>

          <button
            className="secondary logout"
            onClick={logout}
          >
            Logout
          </button>
        </div>
      </header>

      <main className="content">
        <div className="page-heading">
          <div>
            <h2>
              {user.role === "admin"
                ? "Admin Dashboard"
                : "My Tasks"}
            </h2>

            <p className="muted">
              {user.role === "admin"
                ? "Create tasks and manage your team."
                : "View your assigned tasks and progress."}
            </p>
          </div>
        </div>

        <Message error={error} success={success} />

        {/* ================= ADMIN ================= */}

        {user.role === "admin" ? (
          <div className="admin-grid">
            {/* Create task */}

            <section className="panel">
              <div className="section-heading">
                <h3>Create a task</h3>
                <span className="count">NEW</span>
              </div>

              <form onSubmit={createTask}>
                <label>Task title</label>

                <input
                  value={title}
                  onChange={(event) =>
                    setTitle(event.target.value)
                  }
                  placeholder="Implement authentication"
                  required
                />

                <label>Description</label>

                <textarea
                  value={description}
                  onChange={(event) =>
                    setDescription(event.target.value)
                  }
                  placeholder="Describe the task..."
                  rows={4}
                />

                <label>Priority</label>

                <select
                  value={priority}
                  onChange={(event) =>
                    setPriority(
                      event.target.value as TaskPriority
                    )
                  }
                >
                  <option value="low">Low</option>
                  <option value="medium">Medium</option>
                  <option value="high">High</option>
                </select>

                <button
                  disabled={loading || !title.trim()}
                >
                  {loading ? "Creating..." : "Create task"}
                </button>
              </form>
            </section>

            {/* Assign task */}

            <section className="panel">
              <div className="section-heading">
                <h3>Assign a task</h3>

                <span className="count">
                  {staffUsers.length} STAFF
                </span>
              </div>

              <form onSubmit={assignTask}>
                <label>Select task</label>

                <select
                  value={selectedTaskId}
                  onChange={(event) =>
                    setSelectedTaskId(event.target.value)
                  }
                  required
                >
                  <option value="">
                    Choose a task
                  </option>

                  {tasks.map((task) => (
                    <option key={task.id} value={task.id}>
                      {task.title}
                    </option>
                  ))}
                </select>

                <label>Assign to staff member</label>

                <select
                  value={selectedStaffId}
                  onChange={(event) =>
                    setSelectedStaffId(event.target.value)
                  }
                  required
                >
                  <option value="">
                    Select a staff member
                  </option>

                  {staffUsers.map((staff) => (
                    <option
                      key={staff.id}
                      value={staff.id}
                    >
                      {staff.email}
                    </option>
                  ))}
                </select>

                {staffUsers.length === 0 && (
                  <p className="hint">
                    No staff users found. Make sure your
                    database contains users with role staff.
                  </p>
                )}

                <button
                  disabled={
                    loading ||
                    !selectedTaskId ||
                    !selectedStaffId
                  }
                >
                  {loading ? "Assigning..." : "Assign task"}
                </button>
              </form>

              <button
                type="button"
                className="secondary refresh-users"
                onClick={() => loadStaffUsers()}
                disabled={loading}
              >
                Refresh staff list
              </button>
            </section>

            {/* Dashboard stats */}

            <section className="stats-row admin-stats full-width">
              <div className="stat">
                <span>Total created tasks</span>
                <strong>{tasks.length}</strong>
              </div>

              <div className="stat">
                <span>Staff members</span>
                <strong>{staffUsers.length}</strong>
              </div>
            </section>

            {/* Created tasks */}

            <section className="panel full-width">
              <div className="section-heading">
                <h3>Created tasks</h3>

                <span className="count">
                  {tasks.length}
                </span>
              </div>

              {tasks.length === 0 ? (
                <EmptyState
                  text="Create your first task to see it here."
                />
              ) : (
                <TaskList tasks={tasks} />
              )}
            </section>
          </div>
        ) : (
          /* ================= STAFF ================= */

          <section className="panel">
            <div className="section-heading">
              <h3>Assigned to me</h3>

              <button
                className="secondary refresh"
                onClick={() => loadMyTasks()}
                disabled={loading}
              >
                {loading ? "Loading..." : "Refresh tasks"}
              </button>
            </div>

            <div className="stats-row">
              <div className="stat">
                <span>Total assigned</span>
                <strong>{myTasks.length}</strong>
              </div>

              <div className="stat">
                <span>Cache status</span>

                <strong>
                  {cacheHit === null
                    ? "Not loaded"
                    : cacheHit
                    ? "Cache hit"
                    : "Cache miss"}
                </strong>
              </div>
            </div>

            {cacheHit !== null && (
              <div
                className={`cache-banner ${
                  cacheHit ? "hit" : "miss"
                }`}
              >
                <span>
                  {cacheHit
                    ? "Data served from Moka cache"
                    : "Data fetched from SQLite and cached"}
                </span>

                <span>
                  cache.hit: {String(cacheHit)}
                </span>
              </div>
            )}

            {myTasks.length === 0 ? (
              <EmptyState
                text="No tasks have been assigned to you yet."
              />
            ) : (
              <TaskList tasks={myTasks} />
            )}
          </section>
        )}
      </main>
    </div>
  );
}

// ==================== MESSAGE COMPONENT ====================

function Message({
  error,
  success,
}: {
  error: string;
  success: string;
}) {
  if (error) {
    return (
      <div className="message error">
        {error}
      </div>
    );
  }

  if (success) {
    return (
      <div className="message success">
        {success}
      </div>
    );
  }

  return null;
}

// ==================== TASK LIST ====================

function TaskList({ tasks }: { tasks: Task[] }) {
  return (
    <div className="task-list">
      {tasks.map((task) => (
        <div className="task-item" key={task.id}>
          <div className="task-main">
            <h4>{task.title}</h4>

            <p>
              {task.description ||
                "No description provided."}
            </p>

            <small>Task ID: {task.id}</small>
          </div>

          <div className="task-meta">
            <span className={`priority ${task.priority}`}>
              {task.priority}
            </span>

            <span className={`status ${task.status}`}>
              {task.status.replace("_", " ")}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}

// ==================== EMPTY STATE ====================

function EmptyState({ text }: { text: string }) {
  return (
    <div className="empty-state">
      <div className="empty-icon">✓</div>
      <p>{text}</p>
    </div>
  );
}
