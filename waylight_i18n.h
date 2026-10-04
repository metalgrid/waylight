// Minimal C++ helpers for the greeter's UI language. main.rs resolves
// waylight.json and calls these before the QML engine loads; unit tests use
// the catalog lookup to verify the committed .qm content. Every failure is
// reported to the caller and never fatal.
#pragma once

#include <QGuiApplication>
#include <QList>
#include <QString>
#include <QTranslator>
#include <Qt>

// Translators must outlive the event loop; the application owns them and this
// registry lets tests remove them again. The greeter installs at most one.
inline QList<QTranslator *> &waylight_translators()
{
    static QList<QTranslator *> translators;
    return translators;
}

inline bool waylight_install_translator(const QString &path)
{
    auto *translator = new QTranslator(qGuiApp);
    if (!translator->load(path)) {
        delete translator;
        return false;
    }
    if (!QGuiApplication::installTranslator(translator)) {
        delete translator;
        return false;
    }
    waylight_translators().append(translator);
    return true;
}

inline void waylight_set_layout_direction(bool right_to_left)
{
    // Always explicit so the host locale cannot flip the greeter unexpectedly.
    QGuiApplication::setLayoutDirection(right_to_left ? Qt::RightToLeft
                                                      : Qt::LeftToRight);
}

// Loads one compiled catalog and translates a single string directly, without
// an application or installed translators. Empty result: missing catalog or
// untranslated source. Used only by unit tests.
inline QString waylight_translate_catalog(const QString &path,
                                          const QString &context,
                                          const QString &source)
{
    QTranslator translator;
    if (!translator.load(path))
        return QString();
    const QByteArray contextUtf8 = context.toUtf8();
    const QByteArray sourceUtf8 = source.toUtf8();
    return translator.translate(contextUtf8.constData(), sourceUtf8.constData());
}
