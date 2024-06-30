var/global/list/cached_maps = list()

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
		if(what.vars[attribute] == value)
			var/message = "<font color=green>[what.type]</font> at [AREACOORD(what)] - <b>VAR:</b> <font color=red>[attribute] = [isnull(value) ? "null" : (isnum(value) ? value : "\"[value]\"")]</font>"
			log_mapping("DIRTY VAR: [message]")
			dirty_vars += message
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