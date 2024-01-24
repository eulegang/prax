--- @meta _

--- @return nil
function focus() end

--- @param name string
--- @return TargetRef
function target(name) end

--- @class TargetRef
TargetRef = {}

--- @param ... Rule Rules to add to target reference
--- @return TargetRef
function TargetRef:req(...) end

--- @param ... Rule
--- @return TargetRef
function TargetRef:resp(...) end

--- @class Attr
--- @class Rule

--- @param name string
--- @return Attr
function header(name) end

--- @param name string
--- @return Attr
function query(name) end

--- @param attr Attr
--- @param value string
--- @return Rule
function set(attr, value) end
