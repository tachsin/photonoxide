function files = export_fields(run_dir, out_dir, options)
%EXPORT_FIELDS  Writes a photonoxide run's field files again, into another folder.
%   files = photonoxide.export_fields(run_dir, out_dir) writes every field file of the run in
%   run_dir into out_dir, another folder, each written again at the precision and deflate level
%   asked, its meta.json saying so, and returns the files written, a string array. An .h5, or
%   an .npz compressed with Zstandard, is copied as it is. photonoxide.estimate_fields gives
%   their sizes before writing.
%
%   Options: files, the names of those to write ("mode-1-of-1.npz", ...; all of them when
%   empty); precision, "f64" or "f32" (the fields downcast, the grid kept f64); and level,
%   deflate's 0 (stored) to 9 (smallest).
arguments
    run_dir (1,1) string
    out_dir (1,1) string
    options.files (1,:) string = string.empty
    options.precision (1,1) string = "f64"
    options.level (1,1) double = 6
end
files = strings(call(@() py.photonoxide.matlab.export_fields(run_dir, out_dir, ...
    py.list(cellstr(options.files)), options.precision, options.level)));
end
