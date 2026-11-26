# Tauri GNU Probe — Phase 0

هذا نموذج **مؤقت** للتحقق من أن Tauri 2 يُبنى ويرتبط على
`x86_64-pc-windows-gnu` مع MinGW-w64، دون أي اعتماد على MSVC.

- الواجهة ثابتة في `dist/index.html` (بلا npm أو Vite).
- لا توجد أوامر Tauri مخصصة؛ الهدف إثبات البناء والربط وتشغيل WebView2.
- البناء لا يُشغّل محلياً. يُشغّل عبر `.github/workflows/ci.yml`.
- بعد نجاح الفحص، يُحذف هذا المجلد أو يُستبدل بهيكل `src-tauri` النهائي.
- النافذة بلا شريط Windows افتراضي (`decorations: false`) وتستخدم Custom Title
  Bar مرسوماً في HTML، مع أزرار تصغير/تكبير/إغلاق عبر Tauri window API.

## المعاينة بدون تثبيت التطبيق

للمعاينة البصرية فقط (بدون Rust/Tauri):

- افتح `probe/tauri-probe/dist/index.html` مباشرة في المتصفح.
- أو شغّل سيرفر ملفات ثابت بسيط على هذا المجلد إن احتجت، ثم افتحه في المتصفح.

> لا تشغّل `cargo tauri dev` محلياً. بناء الـ exe يتم على GitHub Actions فقط
> ويُحمَّل من صفحة الـ artifacts.

## البناء على CI

```powershell
cargo build --manifest-path probe/tauri-probe/src-tauri/Cargo.toml `
  --target x86_64-pc-windows-gnu
```

## معايير النجاح

1. نجاح `cargo build` على GNU.
2. وجود `daftar-probe.exe` كأثر (artifact).
3. فحص `objdump -p` ليعرض DLLs غير تابعة لـ MinGW قدر الإمكان.
4. تشغيل يدوي على جهاز Windows به WebView2، وظهور صفحة النموذج.

## عند الفشل

إن فشل البناء أو الربط على GNU، يُسجّل الخطأ في CI، وننتقل إلى البديل
الإلزامي `egui/eframe` بعد موافقة المالك.
