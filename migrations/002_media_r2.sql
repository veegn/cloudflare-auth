-- R2 媒体对象键（头像 / 应用图标）
-- 本地/远端旧库均需显式 ALTER；CREATE TABLE IF NOT EXISTS 不会补列。

ALTER TABLE users ADD COLUMN avatar_object_key TEXT;
ALTER TABLE apps ADD COLUMN icon_object_key TEXT;
