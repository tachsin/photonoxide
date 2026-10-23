function files = export_run(run_dir, options)
%EXPORT_RUN  Writes a photonoxide run's tables as CSV files.
%   files = photonoxide.export_run(run_dir) writes every table the run in run_dir has (its
%   n_eff, S-parameters, spectra, study, history, static numbers) as CSV files into the run's
%   own folder, and returns the files written, a string array. A run writes them itself when
%   it ends; this writes them again, or into another folder.
%
%   Options: tables, the names of those to write ("modes", "s_parameters", ...; all of them
%   when empty), and out_dir, the folder to write into (the run's own when empty).
arguments
    run_dir (1,1) string
    options.tables (1,:) string = string.empty
    options.out_dir (1,1) string = ""
end
files = strings(call(@() py.photonoxide.matlab.export_run(run_dir, ...
    py.list(cellstr(options.tables)), options.out_dir)));
end
