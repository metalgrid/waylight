use cxx_qt_build::{CxxQtBuilder, QmlModule};
fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("Waylight").qml_files(["App.qml", "Main.qml"]))
        .file("src/backend.rs")
        .qt_module("QuickControls2")
        .qrc("resources.qrc")
        .build();
}
