function g = device_geometry(kind, options)
%DEVICE_GEOMETRY  A device's geometry: its stack, the polygons on each layer, its ports.
%   g = photonoxide.device_geometry(kind) is the geometry of the device kind at its defaults:
%   g.layers, a struct array bottom to top (name, z_um, material, background, polygons: each
%   polygon's outer outline n-by-2 in micrometres, counterclockwise, its holes, etch_depth_um,
%   empty through the layer, and sidewall_deg), g.substrate, g.cladding and g.ports (name,
%   position_um, direction_deg, width_um, layer).
%
%   Options: values and stack, as photonoxide.device_spectrum takes them.
arguments
    kind (1,1) string
    options.values (1,1) struct = struct()
    options.stack (1,1) string = ""
end
g = jsondecode(char(call(@() py.photonoxide.matlab.device_geometry(kind, ...
    jsonencode(options.values), options.stack))));
end
