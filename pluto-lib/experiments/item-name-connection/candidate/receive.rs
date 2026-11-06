fn preserve_name_removal(value: Option<Tag>, stack: &Stack) -> Option<Tag> {
    crate::pluto_name_removal::receive(value, stack)
}
