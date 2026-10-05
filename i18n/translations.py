#!/usr/bin/env python3
"""Apply the reviewed translations to the .ts catalogs produced by lupdate.

Only messages whose translation is missing or unfinished are filled in, so a
human translator's committed text always wins. Sources without a complete
table entry are reported on stderr but never abort: lrelease drops unfinished
messages and the greeter falls back to the English source at runtime.

Usage: python3 i18n/translations.py   (run from the project root, after lupdate)
"""
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

LANGS = ["zh", "hi", "es", "fr", "ar", "bn", "pt", "ru", "ur", "bg"]

# source text in Main.qml -> translation per language. Short standard UI
# wording; "Wayland"/"Greeter" stay latin where customary for that locale.
TABLE = {
    " (preview only)": {
        "zh": "（仅预览）", "hi": " (केवल पूर्वावलोकन)", "es": " (solo vista previa)",
        "fr": " (aperçu seulement)", "ar": " (معاينة فقط)", "bn": " (শুধুমাত্র প্রিভিউ)",
        "pt": " (somente pré-visualização)", "ru": " (только предпросмотр)",
        "ur": " (صرف پیش نظارہ)", "bg": " (само преглед)",
    },
    "…": {
        "zh": "…", "hi": "…", "es": "…", "fr": "…", "ar": "…",
        "bn": "…", "pt": "…", "ru": "…", "ur": "…", "bg": "…",
    },
    "Authentication response": {
        "zh": "身份验证应答", "hi": "प्रमाणीकरण प्रतिक्रिया", "es": "Respuesta de autenticación",
        "fr": "Réponse d'authentification", "ar": "إجابة المصادقة", "bn": "প্রমাণীকরণের উত্তর",
        "pt": "Resposta de autenticação", "ru": "Ответ аутентификации",
        "ur": "تصدیق کا جواب", "bg": "Отговор за удостоверяване",
    },
    "Cancel": {
        "zh": "取消", "hi": "रद्द करें", "es": "Cancelar", "fr": "Annuler", "ar": "إلغاء",
        "bn": "বাতিল করুন", "pt": "Cancelar", "ru": "Отмена", "ur": "منسوخ کریں", "bg": "Отказ",
    },
    "Choose user": {
        "zh": "选择用户", "hi": "उपयोगकर्ता चुनें", "es": "Elegir usuario", "fr": "Choisir un utilisateur",
        "ar": "اختيار مستخدم", "bn": "ব্যবহারকারী নির্বাচন করুন", "pt": "Escolher usuário",
        "ru": "Выбрать пользователя", "ur": "صارف منتخب کریں", "bg": "Избор на потребител",
    },
    "Closing after authentication cleanup…": {
        "zh": "正在完成身份验证清理并关闭…", "hi": "प्रमाणीकरण सफ़ाई के बाद बंद हो रहा है…",
        "es": "Cerrando tras la limpieza de la autenticación…",
        "fr": "Fermeture après le nettoyage de l'authentification…",
        "ar": "جارٍ الإغلاق بعد تنظيف المصادقة…", "bn": "প্রমাণীকরণ পরিষ্কারের পরে বন্ধ হচ্ছে…",
        "pt": "Encerrando após a limpeza da autenticação…",
        "ru": "Завершение после очистки аутентификации…",
        "ur": "تصدیق کی صفائی کے بعد بند ہو رہا ہے…", "bg": "Затваряне след приключване на удостоверяването…",
    },
    "Connecting; controls are not ready yet.": {
        "zh": "正在连接；控件尚未就绪。", "hi": "कनेक्ट हो रहा है; नियंत्रण अभी तैयार नहीं हैं।",
        "es": "Conectando; los controles aún no están listos.",
        "fr": "Connexion ; les commandes ne sont pas encore prêtes.",
        "ar": "جارٍ الاتصال؛ عناصر التحكم غير جاهزة بعد.",
        "bn": "সংযোগ করা হচ্ছে; নিয়ন্ত্রণগুলি এখনও প্রস্তুত নয়।",
        "pt": "Conectando; os controles ainda não estão prontos.",
        "ru": "Подключение; элементы управления ещё не готовы.",
        "ur": "رابطہ قائم ہو رہا ہے؛ کنٹرولز ابھی تیار نہیں۔",
        "bg": "Свързване; управлението все още не е готово.",
    },
    "Continue": {
        "zh": "继续", "hi": "जारी रखें", "es": "Continuar", "fr": "Continuer", "ar": "متابعة",
        "bn": "চালিয়ে যান", "pt": "Continuar", "ru": "Продолжить", "ur": "جاری رکھیں", "bg": "Продължи",
    },
    "Desktop session": {
        "zh": "桌面会话", "hi": "डेस्कटॉप सत्र", "es": "Sesión de escritorio", "fr": "Session de bureau",
        "ar": "جلسة سطح المكتب", "bn": "ডেস্কটপ সেশন", "pt": "Sessão da área de trabalho",
        "ru": "Сеанс рабочего стола", "ur": "ڈیسک ٹاپ سیشن", "bg": "Сесия на работната среда",
    },
    "Desktop session · Wayland": {
        "zh": "桌面会话 · Wayland", "hi": "डेस्कटॉप सत्र · Wayland", "es": "Sesión de escritorio · Wayland",
        "fr": "Session de bureau · Wayland", "ar": "جلسة سطح المكتب · Wayland",
        "bn": "ডেস্কটপ সেশন · Wayland", "pt": "Sessão da área de trabalho · Wayland",
        "ru": "Сеанс рабочего стола · Wayland", "ur": "ڈیسک ٹاپ سیشن · Wayland",
        "bg": "Сесия на работната среда · Wayland",
    },
    "Disconnected; user switching and power changes are disabled.": {
        "zh": "已断开连接；已禁用切换用户和电源操作。",
        "hi": "कनेक्शन टूटा हुआ है; उपयोगकर्ता बदलना और पावर क्रियाएँ अक्षम हैं।",
        "es": "Desconectado; el cambio de usuario y las acciones de energía están deshabilitadas.",
        "fr": "Déconnecté ; le changement d'utilisateur et les actions d'alimentation sont désactivés.",
        "ar": "انقطع الاتصال؛ تبديل المستخدم وأوامر الطاقة معطّلة.",
        "bn": "সংযোগ বিচ্ছিন্ন; ব্যবহারকারী পরিবর্তন এবং পাওয়ার নিয়ন্ত্রণ নিষ্ক্রিয়।",
        "pt": "Desconectado; a troca de usuário e as ações de energia estão desabilitadas.",
        "ru": "Соединение потеряно; смена пользователя и управление питанием недоступны.",
        "ur": "رابطہ منقطع ہے؛ صارف کی تبدیلی اور بجلی کے اختیارات غیر فعال ہیں۔",
        "bg": "Връзката е прекъсната; смяната на потребител и захранването са забранени.",
    },
    "Enter answer": {
        "zh": "输入应答", "hi": "उत्तर दर्ज करें", "es": "Introducir respuesta", "fr": "Saisir la réponse",
        "ar": "أدخل الإجابة", "bn": "উত্তর লিখুন", "pt": "Digite a resposta", "ru": "Введите ответ",
        "ur": "جواب درج کریں", "bg": "Въведете отговор",
    },
    "Greeter — preview": {
        "zh": "Greeter — 预览", "hi": "ग्रीटर — पूर्वावलोकन", "es": "Greeter — vista previa",
        "fr": "Greeter — aperçu", "ar": "الشاشة الترحيبية — معاينة", "bn": "গ্রিটার — প্রিভিউ",
        "pt": "Greeter — pré-visualização", "ru": "Greeter — предпросмотр",
        "ur": "گریٹر — پیش نظارہ", "bg": "Greeter — преглед",
    },
    "More…": {
        "zh": "更多…", "hi": "और…", "es": "Más…", "fr": "Plus…", "ar": "المزيد…",
        "bn": "আরও…", "pt": "Mais…", "ru": "Ещё…", "ur": "مزید…", "bg": "Още…",
    },
    "More… · Enter a username manually": {
        "zh": "更多… · 手动输入用户名", "hi": "और… · उपयोगकर्ता नाम स्वयं दर्ज करें",
        "es": "Más… · Introducir un usuario manualmente",
        "fr": "Plus… · Saisir un identifiant manuellement",
        "ar": "المزيد… · أدخل اسم المستخدم يدويًا", "bn": "আরও… · ব্যবহারকারীর নাম নিজে লিখুন",
        "pt": "Mais… · Digitar um usuário manualmente",
        "ru": "Ещё… · Ввести имя пользователя вручную",
        "ur": "مزید… · صارف نام خود درج کریں", "bg": "Още… · Въведете потребител ръчно",
    },
    "No": {
        "zh": "否", "hi": "नहीं", "es": "No", "fr": "Non", "ar": "لا",
        "bn": "না", "pt": "Não", "ru": "Нет", "ur": "نہیں", "bg": "Не",
    },
    "Password": {
        "zh": "密码", "hi": "पासवर्ड", "es": "Contraseña", "fr": "Mot de passe", "ar": "كلمة السر",
        "bn": "পাসওয়ার্ড", "pt": "Senha", "ru": "Пароль", "ur": "پاس ورڈ", "bg": "Парола",
    },
    "Power request in progress; controls are disabled.": {
        "zh": "正在执行电源操作；控件已禁用。",
        "hi": "पावर अनुरोध जारी है; नियंत्रण अक्षम हैं।",
        "es": "Solicitud de energía en curso; los controles están deshabilitados.",
        "fr": "Demande d'alimentation en cours ; les commandes sont désactivées.",
        "ar": "طلب الطاقة قيد التنفيذ؛ عناصر التحكم معطّلة.",
        "bn": "পাওয়ার অনুরোধ চলছে; নিয়ন্ত্রণগুলি নিষ্ক্রিয়।",
        "pt": "Solicitação de energia em andamento; os controles estão desabilitados.",
        "ru": "Выполняется запрос питания; элементы управления недоступны.",
        "ur": "بجلی کی درخواست جاری ہے؛ کنٹرولز غیر فعال ہیں۔",
        "bg": "Изпълнява се заявка за захранване; управлението е забранено.",
    },
    "PREVIEW · No system changes": {
        "zh": "预览 · 不更改系统", "hi": "पूर्वावलोकन · कोई सिस्टम बदलाव नहीं",
        "es": "VISTA PREVIA · Sin cambios en el sistema", "fr": "APERÇU · Aucune modification du système",
        "ar": "معاينة · لا تغييرات على النظام", "bn": "প্রিভিউ · সিস্টেমে কোনো পরিবর্তন নেই",
        "pt": "PRÉ-VISUALIZAÇÃO · Sem alterações no sistema",
        "ru": "ПРЕДПРОСМОТР · Без изменений системы",
        "ur": "پیش نظارہ · نظام میں کوئی تبدیلی نہیں", "bg": "ПРЕГЛЕД · Без промени по системата",
    },
    "Restart": {
        "zh": "重启", "hi": "रीस्टार्ट", "es": "Reiniciar", "fr": "Redémarrer", "ar": "إعادة التشغيل",
        "bn": "রিস্টার্ট", "pt": "Reiniciar", "ru": "Перезагрузка", "ur": "ری اسٹارٹ", "bg": "Рестартиране",
    },
    "Restart this computer?": {
        "zh": "重新启动这台计算机？", "hi": "इस कंप्यूटर को पुनः आरंभ करें?",
        "es": "¿Reiniciar este equipo?", "fr": "Redémarrer cet ordinateur ?",
        "ar": "إعادة تشغيل هذا الحاسوب؟", "bn": "এই কম্পিউটার রিস্টার্ট করবেন?",
        "pt": "Reiniciar este computador?", "ru": "Перезагрузить этот компьютер?",
        "ur": "اس کمپیوٹر کو دوبارہ شروع کریں؟", "bg": "Рестартиране на компютъра?",
    },
    "Selected user": {
        "zh": "已选择的用户", "hi": "चयनित उपयोगकर्ता", "es": "Usuario seleccionado",
        "fr": "Utilisateur sélectionné", "ar": "المستخدم المحدد", "bn": "নির্বাচিত ব্যবহারকারী",
        "pt": "Usuário selecionado", "ru": "Выбранный пользователь", "ur": "منتخب صارف",
        "bg": "Избран потребител",
    },
    "Session start is committed; user switching and power changes are disabled.": {
        "zh": "会话启动已提交；已禁用切换用户和电源操作。",
        "hi": "सत्र प्रारंभ प्रतिबद्ध है; उपयोगकर्ता बदलना और पावर क्रियाएँ अक्षम हैं।",
        "es": "El inicio de la sesión está confirmado; el cambio de usuario y las acciones de energía están deshabilitados.",
        "fr": "Le démarrage de la session est confirmé ; le changement d'utilisateur et les actions d'alimentation sont désactivés.",
        "ar": "بدء الجلسة ملزم؛ تبديل المستخدم وأوامر الطاقة معطّلة.",
        "bn": "সেশন শুরু চূড়ান্ত; ব্যবহারকারী পরিবর্তন এবং পাওয়ার নিয়ন্ত্রণ নিষ্ক্রিয়।",
        "pt": "O início da sessão está confirmado; a troca de usuário e as ações de energia estão desabilitadas.",
        "ru": "Запуск сеанса подтверждён; смена пользователя и управление питанием недоступны.",
        "ur": "سیشن کا آغاز طے ہو چکا ہے؛ صارف کی تبدیلی اور بجلی کے اختیارات غیر فعال ہیں۔",
        "bg": "Стартът на сесията е потвърден; смяната на потребител и захранването са забранени.",
    },
    "Shut down this computer?": {
        "zh": "关闭这台计算机？", "hi": "इस कंप्यूटर को बंद करें?",
        "es": "¿Apagar este equipo?", "fr": "Éteindre cet ordinateur ?",
        "ar": "إيقاف تشغيل هذا الحاسوب؟", "bn": "এই কম্পিউটার বন্ধ করবেন?",
        "pt": "Desligar este computador?", "ru": "Выключить этот компьютер?",
        "ur": "اس کمپیوٹر کو بند کریں؟", "bg": "Изключване на компютъра?",
    },
    "Shut Down": {
        "zh": "关机", "hi": "शट डाउन", "es": "Apagar", "fr": "Éteindre", "ar": "إيقاف التشغيل",
        "bn": "শাট ডাউন", "pt": "Desligar", "ru": "Выключение", "ur": "شٹ ڈاؤن", "bg": "Изключване",
    },
    "Sign in": {
        "zh": "登录", "hi": "साइन इन करें", "es": "Iniciar sesión", "fr": "Se connecter",
        "ar": "تسجيل الدخول", "bn": "সাইন ইন করুন", "pt": "Entrar", "ru": "Вход",
        "ur": "سائن اِن کریں", "bg": "Вход",
    },
    "SIGN IN · Wayland": {
        "zh": "登录 · Wayland", "hi": "साइन इन · Wayland", "es": "INICIO DE SESIÓN · Wayland",
        "fr": "CONNEXION · Wayland", "ar": "تسجيل الدخول · Wayland", "bn": "সাইন ইন · Wayland",
        "pt": "ENTRADA · Wayland", "ru": "ВХОД · Wayland", "ur": "سائن اِن · Wayland",
        "bg": "ВХОД · Wayland",
    },
    "Sleep": {
        "zh": "睡眠", "hi": "स्लीप", "es": "Suspender", "fr": "Mettre en veille", "ar": "السكون",
        "bn": "ঘুমিয়ে পড়ুন", "pt": "Suspender", "ru": "Спящий режим", "ur": "سلیپ", "bg": "Приспиване",
    },
    "Submit answer": {
        "zh": "提交应答", "hi": "उत्तर भेजें", "es": "Enviar respuesta", "fr": "Envoyer la réponse",
        "ar": "إرسال الإجابة", "bn": "উত্তর জমা দিন", "pt": "Enviar resposta", "ru": "Отправить ответ",
        "ur": "جواب بھیجیں", "bg": "Изпращане на отговора",
    },
    "Username": {
        "zh": "用户名", "hi": "उपयोगकर्ता नाम", "es": "Usuario", "fr": "Identifiant",
        "ar": "اسم المستخدم", "bn": "ব্যবহারকারীর নাম", "pt": "Usuário", "ru": "Имя пользователя",
        "ur": "صارف نام", "bg": "Потребител",
    },
    "Yes": {
        "zh": "是", "hi": "हाँ", "es": "Sí", "fr": "Oui", "ar": "نعم",
        "bn": "হ্যাঁ", "pt": "Sim", "ru": "Да", "ur": "ہاں", "bg": "Да",
    },
}


def main() -> int:
    root = Path(__file__).resolve().parent
    missing = []
    for path in sorted(root.glob("waylight_*.ts")):
        language = path.stem.removeprefix("waylight_")
        if language not in LANGS:
            continue
        tree = ET.parse(path)
        applied = 0
        for message in tree.iter("message"):
            source = message.find("source")
            translation = message.find("translation")
            if source is None or translation is None:
                continue
            if translation.get("type") != "unfinished" and (translation.text or "").strip():
                continue  # A committed human translation wins.
            entry = TABLE.get(source.text or "")
            if not entry or language not in entry:
                missing.append(f"{path.name}: {source.text!r}")
                continue
            translation.text = entry[language]
            if "type" in translation.attrib:
                del translation.attrib["type"]
            applied += 1
        tree.write(path, encoding="utf-8", xml_declaration=True)
        print(f"{path.name}: {applied} translation(s) applied")
    if missing:
        print("unfinished without table entry:", file=sys.stderr)
        for line in missing:
            print(f"  {line}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
