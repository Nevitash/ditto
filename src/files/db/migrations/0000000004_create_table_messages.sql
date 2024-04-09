CREATE TABLE messages (
  id uuid PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
  origin_subject_id uuid NOT NULL REFERENCES queues(id),  -- Foreign key to Kafka queues table
  original_data bytea NOT NULL,  -- Message content
  created_at timestamp with time zone NOT NULL DEFAULT current_timestamp,
  CONSTRAINT fk_message_origin FOREIGN KEY (origin_subject_id) REFERENCES queues(id)  -- Explicit constraint name
);