// This file provides manually written utility types and such for the BAPI DMM Reader
// and imports the bindings 

// Compatibility for non-ss13
#include "compat.dm"

// Import bindings to the rust library
#include "bapi_bindings.dm"

#define MAP_DMM "dmm"
/**
 * TGM SPEC:
 * TGM is a derevation of DMM, with restrictions placed on it
 * to make it easier to parse and to reduce merge conflicts/ease their resolution
 *
 * Requirements:
 * Each "statement" in a key's details ends with a new line, and wrapped in (...)
 * All paths end with either a comma or occasionally a {, then a new line
 * Excepting the area, who is listed last and ends with a ) to mark the end of the key
 *
 * {} denotes a list of variable edits applied to the path that came before the first {
 * the final } is followed by a comma, and then a new line
 * Variable edits have the form \tname = value;\n
 * Except the last edit, which has no final ;, and just ends in a newline
 * No extra padding is permitted
 * Many values are supported. See parse_constant()
 * Strings must be wrapped in "...", files in '...', and lists in list(...)
 * Files are kinda susy, and may not actually work. buyer beware
 * Lists support assoc values as expected
 * These constants can be further embedded into lists
 *
 * There can be no padding in front of, or behind a path
 *
 * Therefore:
 * "key" = (
 * /path,
 * /other/path{
 *     var = list("name" = 'filepath');
 *     other_var = /path
 *     },
 * /turf,
 * /area)
 *
 */
#define MAP_TGM "tgm"
#define MAP_UNKNOWN "unknown"

/// Returned from parse_map to give some metadata about the map
/datum/bapi_parsed_map
	var/_internal_index = -1

	var/original_path = ""
	var/map_format = MAP_UNKNOWN
	var/key_len = 0
	var/line_len = 0
	var/expanded_y = FALSE
	var/expanded_x = FALSE

	/// Unoffset bounds. Null on parse failure.
	var/list/bounds = list()
	/// Offset bounds. Same as parsed_bounds until load().
	var/list/parsed_bounds = list()

	///any turf in this list is skipped inside of build_coordinate. Lazy assoc list
	var/list/turf_blacklist

	var/loading = FALSE
	var/loaded_warnings = list()

/**
 * Helper and recommened way to load a map file
 * - dmm_file: The path to the map file
 * - x_offset: The x offset to load the map at
 * - y_offset: The y offset to load the map at
 * - z_offset: The z offset to load the map at
 * - crop_map: If true, the map will be cropped to the world bounds
 * - measure_only: If true, the map will not be loaded, but the bounds will be calculated
 * - no_changeturf: If true, the map will not call /turf/AfterChange
 * - x_lower: The minimum x coordinate to load
 * - x_upper: The maximum x coordinate to load
 * - y_lower: The minimum y coordinate to load
 * - y_upper: The maximum y coordinate to load
 * - z_lower: The minimum z coordinate to load
 * - z_upper: The maximum z coordinate to load
 * - place_on_top: Whether to use /turf/proc/PlaceOnTop rather than /turf/proc/ChangeTurf
 * - new_z: If true, a new z level will be created for the map
 */
/proc/load_map(
	dmm_file,
	x_offset = 1,
	y_offset = 1,
	z_offset = 1,
	crop_map = FALSE,
	measure_only = FALSE,
	no_changeturf = FALSE,
	x_lower = -INFINITY,
	x_upper = INFINITY,
	y_lower = -INFINITY,
	y_upper = INFINITY,
	z_lower = -INFINITY,
	z_upper = INFINITY,
	place_on_top = FALSE,
	new_z = FALSE,
)
	if(!(dmm_file in cached_maps))
		cached_maps[dmm_file] = new /datum/bapi_parsed_map(dmm_file)

	var/datum/bapi_parsed_map/parsed_map = cached_maps[dmm_file]
	parsed_map = parsed_map.copy()
	if(!measure_only && !isnull(parsed_map.bounds))
		parsed_map.load(x_offset, y_offset, z_offset, crop_map, no_changeturf, x_lower, x_upper, y_lower, y_upper, z_lower, z_upper, place_on_top, new_z)
	return parsed_map

/datum/bapi_parsed_map/New(tfile)
	if(isnull(tfile))
		return // create a new datum without loading a map
	var/ret = _bapidmm_parse_map_blocking(tfile, src)
	if(!ret)
		CRASH("Failed to load map [tfile], check rust_log.txt")

/datum/bapi_parsed_map/Destroy()
	..()
	// SSatoms.map_loader_stop(REF(src)) // Just in case, I don't want to double up here
	if(turf_blacklist)
		turf_blacklist.Cut()
	parsed_bounds.Cut()
	bounds.Cut()
	return QDEL_HINT_HARDDEL_NOW


/datum/bapi_parsed_map/proc/copy()
	// Avoids duped work just in case
	build_cache()
	var/datum/bapi_parsed_map/newfriend = new()
	// use the same under-the-hood data
	newfriend._internal_index = _internal_index
	newfriend.original_path = original_path
	newfriend.map_format = map_format
	newfriend.key_len = key_len
	newfriend.line_len = line_len
	newfriend.parsed_bounds = parsed_bounds.Copy()
	// Copy parsed bounds to reset to initial values
	newfriend.bounds = parsed_bounds.Copy()
	newfriend.turf_blacklist = turf_blacklist?.Copy()
	// Explicitly do NOT copy `loaded` and `loaded_warnings`
	return newfriend

/datum/bapi_parsed_map/proc/build_cache()
	return

#define MAPLOADING_CHECK_TICK \
	if(TICK_CHECK) { \
		if(loading) { \
			SSatoms.map_loader_stop(REF(src)); \
			stoplag(); \
			SSatoms.map_loader_begin(REF(src)); \
		} else { \
			stoplag(); \
		} \
	}

/datum/bapi_parsed_map/proc/load(
	x_offset = 1,
	y_offset = 1,
	z_offset = 1,
	crop_map = FALSE,
	no_changeturf = FALSE,
	x_lower = -INFINITY,
	x_upper = INFINITY,
	y_lower = -INFINITY,
	y_upper = INFINITY,
	z_lower = -INFINITY,
	z_upper = INFINITY,
	place_on_top = FALSE,
	new_z = FALSE,
)
	Master.StartLoadingMap()
	. = _load_impl(x_offset, y_offset, z_offset, crop_map, no_changeturf, x_lower, x_upper, y_lower, y_upper, z_lower, z_upper, place_on_top, new_z)
	Master.StopLoadingMap()

/datum/bapi_parsed_map/proc/_load_impl(
	x_offset = 1,
	y_offset = 1,
	z_offset = 1,
	crop_map = FALSE,
	no_changeturf = FALSE,
	x_lower = -INFINITY,
	x_upper = INFINITY,
	y_lower = -INFINITY,
	y_upper = INFINITY,
	z_lower = -INFINITY,
	z_upper = INFINITY,
	place_on_top = FALSE,
	new_z = FALSE,
)
	PRIVATE_PROC(TRUE)
	SSatoms.map_loader_begin(REF(src))
	// `loading` var handled by bapidmm
	var/successful =  _bapidmm_load_map(
		src,
		x_offset,
		y_offset,
		z_offset,
		crop_map,
		no_changeturf,
		x_lower,
		x_upper,
		y_lower,
		y_upper,
		z_lower,
		z_upper,
		place_on_top,
		new_z
	)
	SSatoms.map_loader_stop(REF(src))

	if(new_z)
		for(var/z_index in bounds[MAP_MINZ] to bounds[MAP_MAXZ])
			SSmapping.build_area_turfs(z_index)

	if(!no_changeturf)
		var/list/turfs = block(
			locate(bounds[MAP_MINX], bounds[MAP_MINY], bounds[MAP_MINZ]),
			locate(bounds[MAP_MAXX], bounds[MAP_MAXY], bounds[MAP_MAXZ]))
		for(var/turf/T as anything in turfs)
			//we do this after we load everything in. if we don't, we'll have weird atmos bugs regarding atmos adjacent turfs
			T.AfterChange(CHANGETURF_IGNORE_AIR)

	if(expanded_x || expanded_y)
		SEND_GLOBAL_SIGNAL(COMSIG_GLOB_EXPANDED_WORLD_BOUNDS, expanded_x, expanded_y)

	return successful

/datum/bapi_parsed_map/proc/has_warnings()
	if(length(loaded_warnings))
		return TRUE
	return FALSE


// Internal bapi-dmm helpers
/datum/bapi_parsed_map/proc/_bapi_add_warning(warning)
	loaded_warnings += list(warning)

/datum/bapi_parsed_map/proc/_bapi_expand_map(x, y, z, new_z, z_offset)
	if(x > world.maxx)
		expanded_x = TRUE
		if(new_z)
			world.increase_max_x(x, map_load_z_cutoff = z_offset - 1)
		else
			world.increase_max_x(x)
	if(y > world.maxy)
		expanded_y = TRUE
		if(new_z)
			world.increase_max_y(y, map_load_z_cutoff = z_offset - 1)
		else
			world.increase_max_y(y)
	if(z > world.maxz)
		world.increase_max_z(z)

/proc/_bapi_helper_get_world_bounds()
	return list(world.maxx, world.maxy, world.maxz)

/proc/_bapi_helper_text2path(text)
	return text2path(text)

/proc/_bapi_helper_text2file(text)
	return file(text)

/proc/_bapi_create_atom(path, crds)
	set waitfor = FALSE
	. = new path (crds)

/proc/_bapi_new_atom(text_path, turf/crds, list/attributes)
	var/path = text2path(text_path)
	if(attributes != null)
		world.preloader_setup(attributes, path)

	var/atom/instance = _bapi_create_atom(path, crds) // first preloader pass

	if(use_preloader && instance) // second preloader pass for atoms that don't ..() in New()
		world.preloader_load(instance)

/proc/_bapi_create_or_get_area(text_path)
	var/path = text2path(text_path)

	var/area/area_instance = areas_by_type[path]
	if(!area_instance)
		area_instance = new path(null)
		if(!area_instance)
			CRASH("[path] failed to be new'd, what'd you do?")

	return area_instance

/proc/_bapi_handle_area_contain(turf/T, area/A)
	var/area/old_area = T.loc
	LISTASSERTLEN(old_area.turfs_to_uncontain_by_zlevel, T.z, list())
	LISTASSERTLEN(A.turfs_by_zlevel, T.z, list())
	old_area.turfs_to_uncontain_by_zlevel[T.z] += T
	A.turfs_by_zlevel[T.z] += T
	return old_area

/proc/_bapi_create_turf(turf/crds, text_path, list/attributes, place_on_top, no_changeturf)
	var/path = text2path(text_path)
	if(attributes != null)
		world.preloader_setup(attributes, path)

	var/atom/instance
	if(place_on_top)
		instance = crds.load_on_top(path, CHANGETURF_DEFER_CHANGE | (no_changeturf ? CHANGETURF_SKIP : NONE))
	else if(no_changeturf)
		instance = _bapi_create_atom(path, crds)
	else
		instance = crds.ChangeTurf(path, null, CHANGETURF_DEFER_CHANGE)

	if(use_preloader && instance) // second preloader pass for atoms that don't ..() in New()
		world.preloader_load(instance)

/proc/_bapi_add_turf_to_area(area/A, turf/T)
	A.contents.Add(T)

// #undef MAP_DMM
// #undef MAP_TGM
// #undef MAP_UNKNOWN