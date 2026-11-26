-- Seed data shared by every company: currencies, permissions and system roles.
-- The chart of accounts and the first administrator are created by the company
-- setup service, because they need runtime input and password hashing.
--
-- Seed rows are inserted before the audit triggers exist (migration 3), so the
-- bootstrap itself does not pollute the audit chain.

INSERT INTO currencies (code, name_ar, name_en, minor_units, is_active) VALUES
    ('EGP', 'جنيه مصري', 'Egyptian Pound', 2, 1),
    ('USD', 'دولار أمريكي', 'US Dollar', 2, 1),
    ('EUR', 'يورو', 'Euro', 2, 1),
    ('GBP', 'جنيه إسترليني', 'Pound Sterling', 2, 1),
    ('SAR', 'ريال سعودي', 'Saudi Riyal', 2, 1),
    ('AED', 'درهم إماراتي', 'UAE Dirham', 2, 1),
    ('KWD', 'دينار كويتي', 'Kuwaiti Dinar', 3, 1);

INSERT INTO permissions (code, name_ar) VALUES
    ('entries.create', 'إنشاء قيد'),
    ('entries.post', 'ترحيل قيد'),
    ('entries.void', 'إلغاء مسودة'),
    ('accounts.manage', 'إدارة دليل الحسابات'),
    ('reports.view', 'عرض التقارير'),
    ('periods.close', 'إغلاق الفترات'),
    ('users.manage', 'إدارة المستخدمين'),
    ('settings.manage', 'إدارة الإعدادات'),
    ('backup.manage', 'النسخ الاحتياطي');

INSERT INTO roles (code, name_ar, is_system) VALUES
    ('admin', 'مدير', 1),
    ('accountant', 'محاسب', 1),
    ('reviewer', 'مراجع', 1),
    ('viewer', 'مشاهد', 1);

INSERT INTO role_permissions (role_id, permission_code)
SELECT r.id, p.code
FROM roles r
CROSS JOIN permissions p
WHERE r.code = 'admin';

INSERT INTO role_permissions (role_id, permission_code)
SELECT r.id, p.code
FROM roles r
JOIN permissions p ON p.code IN ('entries.create', 'entries.post', 'reports.view')
WHERE r.code = 'accountant';

INSERT INTO role_permissions (role_id, permission_code)
SELECT r.id, p.code
FROM roles r
JOIN permissions p ON p.code IN ('entries.post', 'reports.view')
WHERE r.code = 'reviewer';

INSERT INTO role_permissions (role_id, permission_code)
SELECT r.id, p.code
FROM roles r
JOIN permissions p ON p.code = 'reports.view'
WHERE r.code = 'viewer';

INSERT INTO settings (key, value, updated_at) VALUES
    ('require_dual_approval', '0', '2026-01-01T00:00:00Z'),
    ('theme', 'dark', '2026-01-01T00:00:00Z'),
    ('idle_timeout_minutes', '15', '2026-01-01T00:00:00Z'),
    ('entry_number_prefix', 'JV', '2026-01-01T00:00:00Z'),
    ('auto_backup_enabled', '1', '2026-01-01T00:00:00Z');
