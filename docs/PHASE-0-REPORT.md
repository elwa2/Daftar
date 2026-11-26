# تقرير المرحلة 0

التاريخ: 2026-11-26

## 1) فحص البيئة

| الفحص | الأمر | النتيجة |
|---|---|---|
| Toolchain GNU | `rustup run stable-x86_64-pc-windows-gnu rustc -vV` | Rust 1.99.0، host GNU |
| Rust المباشر | `rustc -vV` | GNU 1.84.1 (PATH) |
| Cargo | `cargo -V` | 1.84.1 مباشر / 1.99.0 عبر rustup |
| GCC | `gcc --version` | MinGW-w64 UCRT 16.2.0 |
| Target | `gcc -dumpmachine` | `x86_64-w64-mingw32` |
| Node | `node -v` | v22.17.0 |
| npm | `npm -v` | 11.6.1 |
| WebView2 | مجلد EdgeWebView | 158.0.4370.33 |
| MSVC | `cl` / `link` | غير موجودين (كما هو مطلوب) |
| المساحة | C/D | 149.4 / 149.2 GB حرة |

الخلاصة: بيئة GNU/MinGW صالحة. لا حاجة لأي تثبيت. لا يوجد `vendor/` ولا
`Cargo.lock` بعد. لا توجد crates لـ Tauri في المخزون المحلي، لذا التحقق المحلي
من Tauri غير ممكن دون تنزيل — والتنزيل ممنوع محلياً.

ملاحظة توافق: أحدث Tauri 2.12.x يتطلب Rust ≥ 1.92. النسخة المتاحة عبر
`rustup` هي 1.99.0، لذا CI سليم. النسخة المباشرة 1.84.1 في PATH أقدم ولا تصلح
لبناء Tauri؛ لا تُستخدم.

فحص إضافي: `webview2-com-sys 0.39.0` (التابع لـ wry/Tauri على Windows) يعلن
دعم `x86_64-pc-windows-gnu` في docs.rs، ويستخدم `WebView2Loader.dll` الديناميكي
على GNU بدل المكتبة الثابتة الخاصة بـ MSVC. هذا مؤشر إيجابي، لكنه ليس بديلاً عن
تشغيل CI الفعلي.

## 2) ما تم إنشاؤه

- `AGENTS.md`: القواعد (أ + ب) بالنص + تعليمات تشغيلية.
- `rust-toolchain.toml`: تثبيت `stable-x86_64-pc-windows-gnu`.
- `.gitignore`: منع `target/` وبيانات الشركات المحلية.
- `docs/ARCHITECTURE.md`: فصل المحرك، Money، الترحيل، الصلاحيات، التدقيق،
  التقارير، الإقفال، CI.
- `docs/SCHEMA.md`: DDL مقترح، Triggers، Views، فهارس، مصفوفة الفرض.
- `probe/tauri-probe`: نموذج Tauri 2 ثابت بلا npm، مع أيقونة وقدرة افتراضية.
- `.github/workflows/ci.yml`: بناء النموذج على Windows GNU ورفع exe.
- `README.md`: نظرة عامة وحالة المرحلة.

## 3) ما لم يُنفّذ عمداً

- لم يُشغّل `cargo build` أو `cargo run` أو `cargo test` محلياً.
- لم يُنشأ `target/`.
- لم تُنزّل أي crate أو حزمة npm.
- لم يُنشأ `vendor/`.
- لم يُنشأ `Cargo.lock` (يُولَّد في CI أو بموافقة).
- لم يبدأ كود `crates/` أو `src-tauri/` النهائي.

## 4) ما تم التحقق منه محلياً

- صحة JSON لملفات Tauri (تم التحقق برمجياً).
- صحة YAML لسير العمل (تم تحليله بـ PyYAML).
- **اختبار دخان SQL** في SQLite بالذاكرة: أُنشئت الجداول والـ Triggers والـ
  Views، وتم:
  - ترحيل قيد متوازن بنجاح.
  - رفض تعديل/حذف قيد مرحّل.
  - رفض قيد غير متوازن.
  - رفض الترحيل بمستخدم هو منشئ القيد عند تفعيل الاعتماد المزدوج.
  - رفض الترحيل في فترة مقفلة.
  - رفض الترحيل على حساب تجميعي.
  - إنشاء سلسلة تدقيق ورفض تعديل `audit_log`.
- لم يُشغّل أي بناء Rust أو Tauri محلياً.

> اختبار SQL استخدم SQLite المضمن في Python 3.12، وليس نسخة `rusqlite`
> النهائية. الغرض فحص الصياغة والمنطق، وستُعاد الاختبارات الكاملة على CI.

## 5) التحقق المطلوب

### على GitHub Actions (بعد الدفع)

1. `CI / Tauri 2 probe (Windows GNU)` ينجح.
2. الأثر `daftar-tauri-gnu-probe` موجود.
3. مخرجات `objdump -p` لا تُظهر اعتماداً على DLLs من MSVC.
4. إن فشل البناء أو الربط: نسجل الخطأ ونقرر الانتقال إلى `egui/eframe`.

### يدوياً على جهازك

1. فك/تشغيل `daftar-probe.exe` من الأثر.
2. تظهر نافذة عربية RTL بها «Tauri IPC متاح».
3. التأكد من وجود WebView2 (موجود مسبقاً).

## 6) قرارات مفتوحة قبل المرحلة 1

1. `posted` نهائي + قيد عكسي، أم السماح بـ `posted → void`؟
   - المقترح: `posted` نهائي؛ `void` للمسودات فقط؛ العكس قيد مرحّل جديد.
2. المستخدمون داخل كل شركة (المقترح) أم حساب عام مشترك؟
3. دعم `minor_units` مختلفة للعملات من البداية أم تثبيت وحدتين؟
4. الموافقة على تنزيل خط Cairo (حجم تقريبي مبدئي: 200–500 KB لملف WOFF2
   العربي، أو أكبر مع الأوزان المتعددة) ووضعه داخل `ui/`.
5. AG Grid Community مبدئياً أم جدول Virtualized خفيف؟
6. اعتماد إضافة `fiscal_period_id` إلى `journal_entries` (المقترح) أم الاكتفاء
   بالتاريخ.

## 7) أوامر مقترحة للدفع

الحالة الحالية: مستودع Git محلي على `main` بلا commits وبلا remote، و`gh` غير
مسجل الدخول. لذلك لا يمكن تشغيل CI من هنا.

```powershell
git add .
git commit -m "phase 0: GNU rules, architecture, schema, Tauri probe"
git remote add origin <repo-url>
git push -u origin main
```

ثم راقب تبويب Actions. لا تُشغّل أي بناء محلي.
