pub fn update(label: &gtk::Label, selected: &[String]) {
    label.set_label(if selected.is_empty() {
        "Nenhuma seleção"
    } else if selected.len() == 1 {
        "1 elemento selecionado"
    } else {
        return label.set_label(&format!("{} elementos selecionados", selected.len()));
    });
}
