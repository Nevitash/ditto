CREATE TABLE queues (
  id uuid PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
  server_id uuid NOT NULL REFERENCES queue_servers(id),
  queue varchar(255) NOT NULL,
  subject varchar(255) NOT NULL,
  CONSTRAINT unique_queue_subject UNIQUE (server_id, queue, subject)
);

CREATE INDEX idx_queue_subject ON queues (server_id, queue, subject);
