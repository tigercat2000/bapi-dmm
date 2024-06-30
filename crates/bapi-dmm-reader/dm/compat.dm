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

/proc/stack_trace(msg)
	CRASH(msg)

/datum/proc/Destroy()
	return

#define QDEL_HINT_HARDDEL_NOW 4