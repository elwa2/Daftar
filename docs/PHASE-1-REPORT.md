# تقرير المرحلة 1 — المحرك وقاعدة البيانات

التاريخ: 2026-11-26

النطاق: `crates/core` + `crates/db` + مولّد triggers التدقيق + وظيفة CI للاختبارات.
لم تُنشأ بعد `services` ولا `reports` ولا `src-tauri` النهائي ولا `ui`.

## 1) ما تم إنشاؤه

### 1.1 `crates/core` (بلا أي اعتماد على SQLite أو Tauri)

| الملف | المحتوى |
|---|---|
| `money.rs` | `Money` عدد صحيح بأصغر وحدة، `parse`/`format`، `ExchangeRate` بمقياس 1,000,000 وتحويل `half away from zero` |
| `clock.rs` | وقت UTC بصيغة ISO-8601 بدون `chrono` |
| `domain.rs` | `Date`، تعدادات نوع الحساب/الطبيعة/حالة القيد، `JournalLine`/`JournalEntry` مع التحقق |
| `context.rs` | `CompanyId` و`Context` وصلاحيات المستخدم |
| `audit.rs` | `audit_hash` القانوني + `GENESIS_HASH` |
| `error.rs` | `CoreError` بأكواد ثابتة ورسائل عربية |

### 1.2 `crates/db`

| الملف | المحتوى |
|---|---|
| `open.rs` | `Database::open` / `open_in_memory`، ضبط PRAGMAs، WAL للملفات فقط |
| `functions.rs` | تسجيل `app_user_id()` و`audit_hash(...)` بـ `SQLITE_INNOCUOUS` |
| `migrate.rs` | Runner داخلي بـ `PRAGMA user_version` داخل `BEGIN IMMEDIATE` |
| `audit.rs` | `verify_chain` و`append_event` |
| `migrations/0001_schema.sql` | 20 جدولاً، 6 views، فهارس، triggers السلامة |
| `migrations/0002_seed.sql` | 7 عملات، 9 صلاحيات، 4 أدوار، إعدادات افتراضية |
| `migrations/0003_audit.sql` | 87 trigger (تدقيق + منع تعديل `audit_log`) |
| `tests/integrity.rs` | 17 اختبار تكامل على المخطط الحقيقي |

### 1.3 أدوات وCI

- `tools/gen_audit_triggers.py`: يولّد `0003_audit.sql` من قائمة الأعمدة، فلا
  تُكتب triggers التدقيق يدوياً.
- `.github/workflows/ci.yml`: وظيفة جديدة `workspace-tests` تشغّل
  `fmt --check` و`clippy -D warnings` و`cargo test --workspace --all-targets`
  على Rust GNU + MinGW UCRT64.

### 1.4 الفرض المفروض في SQLite (وليس في Rust فقط)

- القيد يبدأ `draft` فقط، ولا يُرحَّل إلا إذا: سطران على الأقل، مدين = دائن،
  الفترة والسنة مفتوحتان ومتطابقتان مع التاريخ، كل الحسابات قابلة للترحيل ونشطة،
  الرقم هو التالي في العدّاد، وبيانات الترحيل مكتملة، ومع الاعتماد المزدوج
  المرحّل ≠ المنشئ.
- القيد المرحّل لا يُعدَّل ولا يُحذف، و`void` نهائي، والانتقالات المسموحة
  `draft → posted` و`draft → void` فقط.
- العكس: `source_type='reversal'` + `reversed_entry_id` لقيد مرحّل، وقيد عكسي
  واحد لكل قيد أصلي (فهرس فريد).
- الفترات: داخل السنة، بلا تداخل، ولا تُعدَّل تواريخها بعد وجود قيود، والسنة
  لا تُقفل قبل إغلاق فتراتها، والمغلق نهائي.
- الحسابات: الأب غير قابل للترحيل، النوع يطابق الأب والأبناء، لا دورة، والنوع
  والطبيعة مقفلان بعد وجود سطور.
- `audit_log` append-only، والسلسلة قابلة للتحقق وإثبات أي تلاعب.

## 2) انحرافات عن الوثيقة (تحتاج اعتمادك)

1. **`rusqlite_migration` → runner داخلي.** السبب: الحزمة غير مخزَّنة محلياً،
   والبديل 40 سطراً واضحاً مع `user_version`. لا يغيّر أي سلوك وظيفي.
2. **`chrono` → حساب UTC داخلي.** نفس السبب. `clock::now_iso8601` يغطي حاجتنا
   (طوابع زمنية للتدقيق فقط).
3. **`audit_hash` بثمانية معاملات** بدل `sha256_hex` بسلسلة واحدة. الناتج نفسه
   (نفس الصيغة `a|b|c|...`)، لكن استدعاءه من trigger أوضح ولا يكرر بناء السلسلة.
4. **`rusqlite` مثبَّت على `0.30.0`** (SQLite 3.44.0 مضمَّنة) لأنه المخزَّن محلياً
   والقابل للتحقق الآن. الترقية تحتاج موافقتك على التنزيل.
5. **`0003_audit.sql` مولَّد**، وليس مكتوباً يدوياً. الملف المولَّد مُلتزَم في
   المستودع، والمولّد أداة تطوير فقط.

## 3) ما تم التحقق منه محلياً

### 3.1 Rust (على نسخة مؤقتة خارج المشروع)

- `cargo fmt --all -- --check`: نظيف.
- `cargo check --offline --all-targets`: نجح للـ libs والاختبارات.
- `cargo clippy --offline --workspace --all-targets -- -D warnings`: بلا تحذيرات.
- لم يُشغَّل `cargo test` محلياً (ممنوع بالقواعد)؛ الاختبارات الـ 37 تُشغَّل في CI.

### 3.2 SQL (بايثون، SQLite 3.44)

طُبِّقت الملفات الثلاثة على قاعدة ذاكرة، ونُفِّذ سيناريو مطابق لاختبارات Rust:

1. ترحيل قيد متوازن + سلسلة تدقيق سليمة.
2. منع تعديل/حذف القيد المرحّل.
3. منع إدخال قيد مرحّل مباشرة.
4. رفض غير المتوازن مع تراجع العدّاد.
5. رفض حساب غير قابل للترحيل.
6. الاعتماد المزدوج.
7. ترقيم متسلسل بلا فجوات.
8. العكس المرتبط + منع عكس ثانٍ + رفض عكس مسودة.
9. رفض تاريخ خارج الفترة.
10. `void` نهائي.
11. رفض دورة الحسابات.
12. رفض تداخل الفترات.
13. الفترة المقفلة تمنع الترحيل.
14. السنة لا تُقفل بفترات مفتوحة.
15. `audit_log` append-only.
16. كشف التلاعب بالسلسلة.
17. إنشاء المخطط كاملاً مع `trusted_schema=OFF` (بدون تنفيذ triggers).

كل النتائج: PASS.

> ملاحظة: بايثون لا يستطيع وسم دواله `INNOCUOUS`، لذا فُحص المنطق مع
> `trusted_schema` الافتراضي. تم التأكد من مصدر SQLite 3.44 أن
> `trusted_schema=OFF` يمنع فقط الدوال الموسومة `DIRECT`/`UNSAFE`، وأن دوال
> SQLite المدمجة (`json_object`, `strftime`, `date`) ودوالنا الموسومة
> `INNOCUOUS` تمر. الاختبار الفعلي بهذا الإعداد يجري في CI.

## 4) مشكلة بيئة محلية مكتشفة (لا تؤثر على CI)

مسار GCC عندك يحتوي مسافة:
`G:\Visual Studio\Rust\winlibs-...`. هذا البناء من GCC 16.2 يحسب مساره الحقيقي
ثم يمرّر `default-manifest.o` إلى `ld` دون تنصيص، فيفشل **أي** ربط، حتى
`int main(void){return 0;}`.

- الحل المؤقت المستخدم للتحقق: تمرير
  `-B C:/daftar-mingw64/x86_64-w64-mingw32/lib/` (junction بلا مسافات) إلى
  gcc عبر `CFLAGS` و`RUSTFLAGS`.
- CI لا يتأثر: MSYS2 UCRT64 يُثبَّت في `C:\msys64\ucrt64` بلا مسافات.
- إن أردت لاحقاً بناءً محلياً مريحاً، الحل الدائم هو نقل MinGW إلى مسار بلا
  مسافات (مثلاً `C:\mingw64`) — قرار يخص جهازك ولا أنفّذه دون طلبك.

أوامر إعادة التحقق (كلها على نسخة مؤقتة، وبلا شبكة):

```powershell
# 1) junction بلا مسافات (مرة واحدة)
New-Item -ItemType Junction -Path C:\daftar-mingw64 `
  -Target "G:\Visual Studio\Rust\winlibs-x86_64-posix-seh-gcc-16.2.0-mingw-w64ucrt-14.0.0-r2"

# 2) نسخ المصادر إلى temp + vendor من ~/.cargo/registry/cache (سكربت بايثون)
#    ثم:
$tc = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin"
$env:PATH = "C:\daftar-mingw64\bin;$tc;" + $env:PATH
$env:RUSTC = "$tc\rustc.exe"
$env:CFLAGS = "-BC:/daftar-mingw64/x86_64-w64-mingw32/lib/"
$env:RUSTFLAGS = "-C link-arg=-BC:/daftar-mingw64/x86_64-w64-mingw32/lib/"
$env:CARGO_TARGET_DIR = "$env:TEMP\daftar-check-target"
& "$tc\cargo.exe" fmt --all -- --check
& "$tc\cargo.exe" check --offline --all-targets
& "$tc\cargo.exe" clippy --offline --workspace --all-targets -- -D warnings
```

> `cargo test` لا يُشغَّل محلياً؛ مكانه CI.

## 5) ما لم يُنفَّذ عمداً

- لا `cargo test` محلياً.
- لا `Cargo.lock` في المستودع (سيُولَّد في CI).
- لا `vendor/` ولا `target/` داخل المشروع.
- لا `services` (الترحيل، الصلاحيات، الإقفال) ولا واجهة.

## 6) التحقق المطلوب

### على GitHub Actions (بعد الدفع)

1. `CI / Core + DB (Windows GNU)`: fmt + clippy + 37 اختباراً، كلها خضراء.
2. `CI / Tauri 2 probe (Windows GNU)`: ينجح ويُنتج الأثر.

### يدوياً على جهازك

1. تأكيد قرارات القسم 7 أدناه.
2. عند وصول CI: مراجعة سجل الاختبارات، وتأكيد أن `trusted_schema=OFF` يعمل مع
   الدوال الموسومة.

## 7) قرارات تنتظر اعتمادك

1. `posted` نهائي + قيد عكسي، و`void` للمسودات فقط.
2. المستخدمون داخل قاعدة كل شركة.
3. `minor_units` من 0 إلى 6 لكل عملة.
4. إضافة `Cargo.lock` إلى المستودع (يُنصح به) — يحتاج موافقتك لأن القاعدة
   تمنع تعديله دون طلب صريح.
5. خط Cairo وAG Grid (يُحسمان في مرحلة الواجهة).
