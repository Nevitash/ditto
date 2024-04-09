CREATE TABLE queue_servers (
  id uuid PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
  server_name varchar(255) NOT NULL UNIQUE,
  config jsonb DEFAULT '{}'::jsonb NOT NULL
);

CREATE INDEX idx_queue_servers_server_name ON queue_servers(server_name);