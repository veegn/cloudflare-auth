-- 迁移：apps.icon_url（schema.sql 的 CREATE TABLE IF NOT EXISTS 不会给旧库加列）
-- 本地： npm run db:schema:local 后仍缺列时执行本文件
-- 远端： npx wrangler d1 execute cloudflare-auth-db --remote --file=./migrations/001_apps_icon_url.sql
ALTER TABLE apps ADD COLUMN icon_url TEXT;
