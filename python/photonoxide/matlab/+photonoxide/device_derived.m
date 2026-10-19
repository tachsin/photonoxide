function f = device_derived(kind, options)
%DEVICE_DERIVED  A device's figures at a fidelity.
%   f = photonoxide.device_derived(kind) is a struct array of the figures of the device kind at
%   its defaults by its cheapest model: f(k).name, f(k).symbol (e.g. "n_eff"), f(k).value,
%   f(k).unit, f(k).fidelity and f(k).grid.
%
%   Options: values, fidelity, stack and compact, as photonoxide.device_spectrum takes them.
arguments
    kind (1,1) string
    options.values (1,1) struct = struct()
    options.fidelity (1,1) string = ""
    options.stack (1,1) string = ""
    options.compact (1,1) string = ""
end
f = jsondecode(char(call(@() py.photonoxide.matlab.device_derived(kind, ...
    jsonencode(options.values), options.fidelity, options.stack, options.compact))));
end
