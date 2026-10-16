function d = devices()
%DEVICES  The devices of photonoxide's library.
%   d = photonoxide.devices() is a struct array, one element per device: d(k).kind (what the
%   device functions take, e.g. "directional coupler"), d(k).about, d(k).parameters (a struct
%   array: name, unit, default, min, max), d(k).fidelities (its models, cheapest first:
%   fidelity, name, write_up, papers, error, validity) and d(k).ports.
d = jsondecode(char(call(@() py.photonoxide.matlab.devices())));
end
