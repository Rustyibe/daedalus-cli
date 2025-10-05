-- Sample tables for testing the Daedalus CLI
-- This will be executed when the PostgreSQL container starts

-- Create a users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create a projects table
CREATE TABLE IF NOT EXISTS projects (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    owner_id INTEGER REFERENCES users(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create a tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id SERIAL PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    assigned_to INTEGER REFERENCES users(id),
    status VARCHAR(20) DEFAULT 'pending',
    priority INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create an api_keys table for storing API keys
CREATE TABLE IF NOT EXISTS api_keys (
    id SERIAL PRIMARY KEY,
    key_value VARCHAR(255) UNIQUE NOT NULL,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    permissions TEXT[] DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP
);

-- Insert sample data
INSERT INTO users (username, email) VALUES 
    ('admin', 'admin@example.com'),
    ('developer1', 'dev1@example.com'),
    ('tester1', 'test1@example.com')
ON CONFLICT (username) DO NOTHING;

INSERT INTO projects (name, description, owner_id) VALUES 
    ('Daedalus CLI', 'A command line interface for managing projects', 1),
    ('Data Analysis Tool', 'Tool for analyzing user data', 2)
ON CONFLICT (name) DO NOTHING;

INSERT INTO tasks (title, description, project_id, assigned_to, status, priority) VALUES 
    ('Implement config module', 'Create module for handling configuration', 1, 2, 'completed', 2),
    ('Write tests', 'Write unit and integration tests', 1, 3, 'in-progress', 1),
    ('Documentation', 'Create user documentation', 2, 2, 'pending', 3)
ON CONFLICT (title) DO NOTHING;

INSERT INTO api_keys (key_value, user_id, name, permissions) VALUES 
    ('sk-1234567890abcdef', 1, 'Admin API Key', ARRAY['read', 'write', 'delete']),
    ('sk-0987654321fedcba', 2, 'Developer API Key', ARRAY['read', 'write'])
ON CONFLICT (key_value) DO NOTHING;

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_projects_owner ON projects(owner_id);
CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned ON tasks(assigned_to);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);