CREATE TABLE steps (
  id uuid PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
  message_id uuid NOT NULL REFERENCES messages(id),
  step_number int,
  data bytea,  -- Optional: Stores modified message content after this step
  started_at timestamp with time zone NOT NULL DEFAULT current_timestamp,
  finished_at timestamp with time zone,  -- Optional: Timestamp when the step is finished
  state_id uuid REFERENCES states(id)  -- Optional: State within this step (pending, processing, done, error)
);

CREATE INDEX idx_step_message_step_number ON steps(message_id, step_number);  -- Combined index