var/global/list/cached_maps = list()

#define AREACOORD(src) "[src ? "[src.x][src.y][src.z]" : "nonexistent location"]"
#define INFINITY 1e31

// Maploader bounds indices
/// The maploader index for the maps minimum x
#define MAP_MINX 1
/// The maploader index for the maps minimum y
#define MAP_MINY 2
/// The maploader index for the maps minimum z
#define MAP_MINZ 3
/// The maploader index for the maps maximum x
#define MAP_MAXX 4
/// The maploader index for the maps maximum y
#define MAP_MAXY 5
/// The maploader index for the maps maximum z
#define MAP_MAXZ 6
#define QDEL_HINT_HARDDEL_NOW 4
#define TICK_CHECK FALSE
#define CHANGETURF_DEFER_CHANGE (1<<0)
#define CHANGETURF_IGNORE_AIR (1<<1) // This flag prevents changeturf from gathering air from nearby turfs to fill the new turf with an approximation of local air
#define CHANGETURF_SKIP (1<<3) // A flag for PlaceOnTop to just instance the new turf instead of calling ChangeTurf. Used for uninitialized turfs NOTHING ELSE
#define ALL (~0) //For convenience.
#define NONE 0

/proc/stack_trace(msg)
	CRASH(msg)

/datum/proc/Destroy()
	return


/proc/log_mapping(message)
	world.log << message

///Copies a list, and all lists inside it recusively
///Does not copy any other reference type
/proc/deep_copy_list(list/inserted_list)
	if(!islist(inserted_list))
		return inserted_list
	. = inserted_list.Copy()
	for(var/i in 1 to inserted_list.len)
		var/key = .[i]
		if(isnum(key))
			// numbers cannot ever be associative keys
			continue
		var/value = .[key]
		if(islist(value))
			value = deep_copy_list(value)
			.[key] = value
		if(islist(key))
			key = deep_copy_list(key)
			.[i] = key
			.[key] = value

var/global/use_preloader = FALSE
var/global/list/_preloader_attributes = null
var/global/_preloader_path = null

/world/proc/preloader_setup(list/the_attributes, path)
	if(the_attributes.len)
		use_preloader = TRUE
		_preloader_attributes = the_attributes
		_preloader_path = path

/world/proc/preloader_load(atom/what)
	use_preloader = FALSE
	var/list/attributes = _preloader_attributes
	for(var/attribute in attributes)
		var/value = attributes[attribute]
		if(islist(value))
			value = deep_copy_list(value)
		#ifdef TESTING
		// if(what.vars[attribute] == value)
			// var/message = "<font color=green>[what.type]</font> at [AREACOORD(what)] - <b>VAR:</b> <font color=red>[attribute] = [isnull(value) ? "null" : (isnum(value) ? value : "\"[value]\"")]</font>"
			// world.log << "DIRTY VAR: [message]"
			// dirty_vars += message
		#endif
		what.vars[attribute] = value

/atom/New(loc, ...)
	//atom creation method that preloads variables at creation
	if(use_preloader && src.type == _preloader_path)//in case the instanciated atom is creating other atoms in New()
		world.preloader_load(src)

var/global/areas_by_type = list()

/area
	var/list/turfs_to_uncontain_by_zlevel = list()
	var/list/turfs_by_zlevel = list()

/area/New()
	. = ..()
	areas_by_type[type] = src

/turf/proc/on_change_area(area/old_area, area/new_area)
	return

/turf/proc/ChangeTurf(path, list/new_baseturfs, flags)
	new path(src)

/turf/proc/load_on_top(turf/added_layer, flags)
	new added_layer(src)

///Ensures the length of a list is at least I, prefilling it with V if needed. if V is a proc call, it is repeated for each new index so that list() can just make a new list for each item.
#define LISTASSERTLEN(L, I, V...) \
	if (length(L) < I) { \
		var/_OLD_LENGTH = length(L); \
		L.len = I; \
		/* Convert the optional argument to a if check */ \
		for (var/_USELESS_VAR in list(V)) { \
			for (var/_INDEX_TO_ASSIGN_TO in _OLD_LENGTH+1 to I) { \
				L[_INDEX_TO_ASSIGN_TO] = V; \
			} \
		} \
	}

// These paths need to exist even if they cannot be spawned
/turf/template_noop
/area/template_noop

// Subsystem stuff
var/global/datum/controller/subsystem/atoms/SSatoms = new()

/datum/controller/subsystem/atoms
/datum/controller/subsystem/atoms/proc/map_loader_begin(source)
/datum/controller/subsystem/atoms/proc/map_loader_stop(source)

var/global/datum/controller/master/Master = new()
/datum/controller/master
/datum/controller/master/proc/StartLoadingMap()
/datum/controller/master/proc/StopLoadingMap()

var/global/datum/controller/subsystem/mapping/SSmapping = new()
/datum/controller/subsystem/mapping
/datum/controller/subsystem/mapping/proc/build_area_turfs(z_index)

/world/proc/increase_max_x(new_maxx, map_load_z_cutoff = 0)
	if(new_maxx <= maxx)
		return
	maxx = new_maxx
	// world.log << "increase_max_x [maxx]"

/world/proc/increase_max_y(new_maxy, map_load_z_cutoff = 0)
	if(new_maxy <= maxy)
		return
	maxy = new_maxy
	// world.log << "increase_max_y [maxy]"

/world/proc/incrementMaxZ()
	maxz++
	// world.log << "incrementMaxZ to [maxz]"

#define PRIVATE_PROC(X)
/// sent after world.maxx and/or world.maxy are expanded: (has_exapnded_world_maxx, has_expanded_world_maxy)
#define COMSIG_GLOB_EXPANDED_WORLD_BOUNDS "!expanded_world_bounds"

#define SEND_GLOBAL_SIGNAL(sigtype, arguments...) world.log << "signal [sigtype]"

/// Takes a datum as input, returns its ref string
#define text_ref(datum) ref(datum)

/**
 * \ref behaviour got changed in 512 so this is necesary to replicate old behaviour.
 * If it ever becomes necesary to get a more performant REF(), this lies here in wait
 * #define REF(thing) (thing && isdatum(thing) && (thing:datum_flags & DF_USE_TAG) && thing:tag ? "[thing:tag]" : text_ref(thing))
**/
/proc/REF(input)
	// if(isdatum(input))
	// 	var/datum/thing = input
	// 	if(thing.datum_flags & DF_USE_TAG)
	// 		if(!thing.tag)
	// 			stack_trace("A ref was requested of an object with DF_USE_TAG set but no tag: [thing]")
	// 			thing.datum_flags &= ~DF_USE_TAG
	// 		else
	// 			return "\[[url_encode(thing.tag)]\]"
	return text_ref(input)

//If you modify this function, ensure it works correctly with lateloaded map templates.
/turf/proc/AfterChange(flags, oldType) //called after a turf has been replaced in ChangeTurf()

///Increases delay as the server gets more overloaded, as sleeps aren't cheap and sleeping only to wake up and sleep again is wasteful
#define DELTA_CALC max(((max(TICK_USAGE, world.cpu) / 100) * 1), 1)

#define CEILING(x, y) ( -round(-(x) / (y)) * (y) )
#define DS2TICKS(DS) ((DS)/world.tick_lag)


#define MAPTICK_LAST_INTERNAL_TICK_USAGE (world.map_cpu)
#define TICK_BYOND_RESERVE 2
/// for general usage of tick_usage
#define TICK_USAGE world.tick_usage
#define TICK_LIMIT_RUNNING (100 - TICK_BYOND_RESERVE - MAPTICK_LAST_INTERNAL_TICK_USAGE)
/// Returns true if tick_usage is above the limit
#define TICK_CHECK ( TICK_USAGE > TICK_LIMIT_RUNNING )
/// runs stoplag if tick_usage is above the limit
#define CHECK_TICK ( TICK_CHECK ? stoplag() : 0 )

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

///returns the number of ticks slept
/proc/stoplag(initial_delay)
	// do nothing
	return

/world/Reboot()
	_bapidmm_clear_map_data()
	. = ..()

/world/Del()
	_bapidmm_clear_map_data()
	. = ..()