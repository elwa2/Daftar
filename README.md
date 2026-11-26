# Daftar — تطبيق محاسبي عربي (Rust + Tauri 2 + SQLite)

> **الحالة: المرحلة 1 مكتملة محلياً؛ المرحلة 2 بدأت.** `core` و`db` و`services`
> موجودة، وأُضيف هيكل تطبيق Tauri فعلي في `src-tauri` يربط `ui/` بأوامر Rust.
> المعاينة البصرية ما زالت ممكنة بفتح `ui/index.html` في المتصفح (بيانات تجريبية)،
> أما الربط الحقيقي بقاعدة SQLite فيظهر داخل تطبيق Tauri. لا بناء Rust محلي؛
> الـ exe عبر GitHub Actions artifacts.
> المشروع مصمم لـ Windows، بناء GNU/MinGW فقط، وواجهة عربية RTL.

## ما هو Daftar؟

تطبيق محاسبة سطح مكتب محلي بنظام القيد المزدوج:

- دليل حسابات شجري، قيود يومية، أستاذ، ميزان مراجعة، مركز مالي، قائمة دخل.
- خزينة وبنوك ومصروفات، عملاء وموردون، ثم مخزون وموظفون.
- SQLite لكل شركة، WAL، ترحيلات، Triggers، وسلسلة تدقيق hash.
- واجهة Tauri 2 (Svelte + TypeScript) عربية RTL بثيم داكن افتراضي.

## قواعد البناء (ملزمة)

- **لا بناء محلي.** لا `cargo build/run`، ولا `cargo tauri build/dev`، ولا
  `npm run tauri`.
- **لا تحميل اعتمادات محلياً.** لا `cargo fetch/update` ولا `npm install` دون
  موافقة. لا `vendor/` قبل موافقة على الحجم.
- **لا `target/` داخل المشروع.** أي `cargo check` مسموح يكون `--offline` مع
  `CARGO_TARGET_DIR=$env:TEMP\acc-target` ثم يُحذف.
- **البناء والاختبارات على GitHub Actions فقط.**
- التفاصيل الكاملة في [AGENTS.md](AGENTS.md).

## البيئة التي فُحصت

| العنصر | النتيجة |
|---|---|
| Rustup toolchain GNU | `stable-x86_64-pc-windows-gnu` — Rust 1.99.0 |
| Rust مباشر في PATH | GNU 1.84.1 (نسخة أقدم، لا تُستخدم للتحقق النهائي) |
| GCC | MinGW-w64 UCRT 16.2.0 (`x86_64-w64-mingw32`) |
| Node.js | 22.17.0 LTS |
| WebView2 | 158.0.4370.33 |
| MSVC / Visual Studio | غير مستخدم |
| المساحة الحرة | C: 149.4 GB، D: 149.2 GB |

> ملاحظة: `rust-toolchain.toml` يثبت GNU، لكن `rustc` المباشر في PATH قد
> يتجاوز rustup. استخدم `rustup run stable-x86_64-pc-windows-gnu ...` عند
> الحاجة للتحقق المحلي.

## تطبيق Tauri والواجهة

- **المعاينة بدون تثبيت:** افتح [ui/index.html](ui/index.html) مباشرة في المتصفح.
  سترى شاشات الحسابات/اليومية/التقارير ببيانات تجريبية، وشريط العنوان المخصص
  (أزرار النافذة تتعطل خارج Tauri).
- **تطبيق Tauri الفعلي:** `src-tauri/` يحمّل `ui/` ويعرّف أوامر Rust مربوطة بـ
  `crates/services`، منها: `bootstrap`, `list_accounts`, `list_journal`,
  `get_journal_entry`, `create_draft`, `post_draft`, `void_draft`,
  `trial_balance`, `income_statement`, `balance_sheet_report`.
- **بناء exe:** عبر GitHub Actions فقط (`.github/workflows/ci.yml`) كأثر
  `daftar-tauri-gnu`. لا يُبنى محلياً.
- **مرجع alyumia:** تم أخذ اتجاهات التقارير/اليومية كتوجيه UX فقط؛ التنفيذ في دفتر
  مبني على Rust/SQLite/Triggers وليس نسخاً من كود المشروع المرجعي.

## المرحلة 1: المحرك وقاعدة البيانات

```text
crates/
├─ core/   Money + ExchangeRate + Date + التحقق + Context + audit_hash + clock
└─ db/     فتح القاعدة + PRAGMAs + دوال SQL + الترحيلات + triggers + verify_chain
```

- `rusqlite 0.30` (SQLite 3.44.0 مضمَّنة) مع `trusted_schema=OFF` ودوال
  `app_user_id()`/`audit_hash(...)` موسومة `INNOCUOUS`.
- 20 جدولاً و6 views و87 trigger؛ كل قواعد السلامة مفروضة في SQLite نفسها.
- 20 اختبار وحدة في `core` و17 اختبار تكامل في `crates/db/tests/integrity.rs`،
  تُشغَّل كلها في CI فقط.
- تحقق محلي منفَّذ: `fmt --check`, `check --offline`, `clippy -D warnings` على
  نسخة مؤقتة، إضافة إلى 17 فحص SQL بايثون.
- التفاصيل والانحرافات: [تقرير المرحلة 1](docs/PHASE-1-REPORT.md).

## الوثائق

- [المعمارية](docs/ARCHITECTURE.md)
- [مخطط قاعدة البيانات](docs/SCHEMA.md)
- [تقرير المرحلة 0](docs/PHASE-0-REPORT.md)
- [تقرير المرحلة 1](docs/PHASE-1-REPORT.md)

## الهيكل المستهدف

```text
accounting-app/
├─ crates/
│  ├─ core/      Money + قواعد القيد
│  ├─ db/        SQLite + الترحيلات + Triggers
│  ├─ services/  القيود + الترحيل + الصلاحيات + الإقفال
│  ├─ reports/   الأستاذ + الميزان + القوائم
│  ├─ auth/      Argon2 + الأدوار + الجلسات
│  └─ io/        CSV/Excel + النسخ الاحتياطي
├─ src-tauri/    أوامر رقيقة
├─ ui/           Svelte + TS + AG Grid
├─ vendor/       عند الحاجة فقط وبموافقة
├─ .github/workflows/
├─ AGENTS.md
└─ tests/
```

## المتطلبات

- Windows 10/11 مع WebView2 Runtime.
- عند التوزيع: ZIP محمول بلا تثبيت، وNSIS اختياري لاحقاً.
- العربية وRTL والطباعة النظيفة أولوية.

## الخطوة التالية

1. اعتمد قرارات القسم 7 في [تقرير المرحلة 1](docs/PHASE-1-REPORT.md).
2. ادفع المستودع إلى GitHub: تعمل وظيفتا CI (اختبارات workspace + نموذج Tauri).
3. بعد نجاح CI: تبدأ المرحلة 2 (`services`: إنشاء القيد، الترحيل، الصلاحيات،
   الإقفال، النسخ الاحتياطي) ثم الواجهة.
