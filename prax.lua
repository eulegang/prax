--- @meta _

--- @return nil
--- only focus the proxy on request that match a target ref
function focus() end

--- @param name string
--- @return TargetRef
--- target a host for proxy rules
function target(name) end

--- @class TargetRef
TargetRef = {}

--- @param ... Rule Rules to add to target reference
--- @return TargetRef
--- add request rules to the current target
function TargetRef:req(...) end

--- @param ... Rule
--- @return TargetRef
--- add response rules to the current target
function TargetRef:resp(...) end

--- @class Attr
--- An attribute of a response or request

--- @class Rule
--- A rule that can be applied during the processing
--- of a request or repsonse

--- @param name string
--- @return Attr
--- Identifies a header in a request or response
function header(name) end

--- @param name string
--- @return Attr
--- Identifies a query in a request
function query(name) end

--- @param attr Attr
--- @param value string
--- @return Rule
--- set a value for a given Attr
function set(attr, value) end

--- @param attr Attr
--- @param transform string | fun(string): string
--- @return Rule
---
--- substitute a value for a given Attr
function sub(attr, transform) end

--- @type Rule
dump = nil

--- @type Rule
intercept = nil

--- @type Attr
method = nil

--- @type Attr
status = nil

--- @type Attr
path = nil

--- @type Attr
body = nil
