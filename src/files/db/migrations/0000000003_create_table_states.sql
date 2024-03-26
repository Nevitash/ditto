CREATE TABLE states (
  id uuid PRIMARY KEY NOT NULL DEFAULT uuid_generate_v4(),
  name varchar(255) NOT NULL UNIQUE,
  description text
);