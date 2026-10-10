function modes = vector_modes(x_um, y_um, permittivity, wavelength_um, options)
%VECTOR_MODES  Full-vector modes of a waveguide's cross-section, with their fields.
%   modes = photonoxide.vector_modes(x_um, y_um, permittivity, wavelength_um) is the
%   fundamental mode of the cross-section on the grid with nodes x_um (nx + 1 of them,
%   increasing, micrometres) and y_um (ny + 1), each cell's relative permittivity in
%   permittivity, ny-by-nx: permittivity(j, i) fills the cell between x_um(i), x_um(i + 1),
%   y_um(j) and y_um(j + 1); real, or complex with a positive imaginary part for loss.
%
%   Each mode has effective_index (complex), te_fraction (near 1 for a TE-like mode), x_um and
%   y_um (the cells' centres) and its fields ex, ey, ez, hx, hy and hz, each ny-by-nx: ex(j, i)
%   is E_x at (x_um(i), y_um(j)).
%
%   Options: count (how many modes, 1), near_index (the modes nearest this effective index; the
%   highest index of the cross-section by default), boundaries (the west, east, south and north
%   edges: "zero", "electric" or "magnetic"), pml_um (PMLs inside the same edges, micrometres)
%   and pml_strength, and timeout_s (600; past it, the error photonoxide:Timeout).
arguments
    x_um double
    y_um double
    permittivity double
    wavelength_um (1,1) double
    options.count (1,1) double = 1
    options.near_index (1,1) double
    options.boundaries (1,4) string = ["zero" "zero" "zero" "zero"]
    options.pml_um (1,4) double = [0 0 0 0]
    options.pml_strength (1,1) double = 0
    options.timeout_s (1,1) double = 600
end
near = py.None;
if isfield(options, 'near_index')
    near = options.near_index;
end
r = call(@() py.photonoxide.matlab.vector_modes(pylist(x_um), pylist(y_um), ...
    pylist(real(permittivity)), pylist(imag(permittivity)), wavelength_um, ...
    int64(options.count), near, py.list(cellstr(options.boundaries)), ...
    pylist(options.pml_um), options.pml_strength, options.timeout_s));
r = cell(r);
modes = struct([]);
for k = 1:numel(r)
    d = struct(r{k});
    n = double(d.effective_index);
    modes(k).effective_index = complex(n(1), n(2));
    modes(k).te_fraction = double(d.te_fraction);
    modes(k).x_um = double(d.x_um);
    modes(k).y_um = double(d.y_um);
    for name = ["ex" "ey" "ez" "hx" "hy" "hz"]
        modes(k).(name) = unflat(d.(name));
    end
end
end
