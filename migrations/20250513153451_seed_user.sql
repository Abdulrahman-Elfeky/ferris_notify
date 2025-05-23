-- Add migration script here
INSERT INTO users (
  user_id, username, password_hash
) VALUES (
  'de8868c3-47e2-46f3-9ff6-1f20621a7544',
  'admin',
  '$argon2id$v=19$m=15000,t=2,p=1$3fk09oGC6RErRynnD6/aeA$ZFwoCfSO97xVPbSz461STCWnDNuwKpMAhUlbZ3BO9KM'
);
