-- PostgreSQL initialization script
-- This script runs when the PostgreSQL container starts up
-- It creates the keycloak database for Keycloak to use

-- Create the keycloak database
CREATE DATABASE keycloak;

-- Grant privileges to the xzepr user for the keycloak database
GRANT ALL PRIVILEGES ON DATABASE keycloak TO xzepr;

-- Connect to keycloak database and ensure proper schema setup
\c keycloak;

-- Grant schema privileges
GRANT ALL ON SCHEMA public TO xzepr;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO xzepr;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO xzepr;

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO xzepr;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO xzepr;

-- Log completion
\echo 'Keycloak database setup completed successfully'
