# BiomeManager.gd
extends Node3D

const BIOME_SCALE = 100.0

enum BiomeType {
	CORAL_REEF,
	SANDY_BOTTOM,
	ROCKY_OUTCROP,
	KELP_FOREST,
	VOLCANIC_VENT
}

class BiomeData:
	var noise: FastNoiseLite # main noise
	var weight_noise: FastNoiseLite # Blending noise
	var height_multiplier: float
	var blend_start: float
	var blend_end: float
	
	func _init(terrain_noise: FastNoiseLite, blend_noise: FastNoiseLite, h_mult: float, b_start: float, b_end: float):
		noise = terrain_noise
		weight_noise = blend_noise
		height_multiplier = h_mult
		blend_start = b_start
		blend_end = b_end


var biomes = {}
var cache = {}
const CACHE_SIZE = 1000

func _enter_tree():
	# We don't initialize biomes here anymore since we wait for noise from TerrainManager
	print("BiomeManager: Entering tree")
	# Initialize empty dictionary
	biomes = {}
	cache = {}

func _ready():
	# Just verify that biomes get initialized
	print("BiomeManager: Ready called")
	if biomes.is_empty():
		print("BiomeManager: Warning - Biomes not yet initialized, waiting for noise initialization")
	else:
		print("BiomeManager: Biomes initialized, count:", biomes.size())


func initialize_biomes_with_noise(
	coral_n: FastNoiseLite, coral_weight: FastNoiseLite,
	sand_n: FastNoiseLite, sand_weight: FastNoiseLite,
	rock_n: FastNoiseLite, rock_weight: FastNoiseLite,
	kelp_n: FastNoiseLite, kelp_weight: FastNoiseLite,
	volcanic_n: FastNoiseLite, volcanic_weight: FastNoiseLite
):
	biomes[BiomeType.CORAL_REEF] = BiomeData.new(coral_n, coral_weight, 0.3, 0.0, 0.3)
	biomes[BiomeType.SANDY_BOTTOM] = BiomeData.new(sand_n, sand_weight, 0.2, 0.2, 0.4)
	biomes[BiomeType.ROCKY_OUTCROP] = BiomeData.new(rock_n, rock_weight, 0.6, 0.3, 0.6)
	biomes[BiomeType.KELP_FOREST] = BiomeData.new(kelp_n, kelp_weight, 0.5, 0.4, 0.7)
	biomes[BiomeType.VOLCANIC_VENT] = BiomeData.new(volcanic_n, volcanic_weight, 0.8, 0.7, 1.0)

func get_sections_for_chunk(chunk_coord: Vector3, size: int) -> Dictionary:
	var cache_key = str(chunk_coord)
	if cache.has(cache_key):
		return cache[cache_key]
	
	var sections = {}
	for biome_type in biomes:
		sections[biome_type] = _generate_section_data(chunk_coord, size, biomes[biome_type])
	
	# Cache management
	if cache.size() > CACHE_SIZE:
		var oldest_key = cache.keys()[0]
		cache.erase(oldest_key)
	
	cache[cache_key] = sections
	return sections

func _generate_section_data(chunk_coord: Vector3, size: int, biome_data: BiomeData) -> Dictionary:
	var section = {
		"heights": [],
		"weights": []
	}
	
	var world_x = chunk_coord.x * size
	var world_z = chunk_coord.z * size
	
	# Pre-calculate heights and weights for the entire chunk
	for x in range(size):
		section.heights.append([])
		section.weights.append([])
		for z in range(size):
			var noise_x = (world_x + x) / BIOME_SCALE
			var noise_z = (world_z + z) / BIOME_SCALE
			
			var height = (biome_data.noise.get_noise_2d(noise_x, noise_z) + 1.0) * 0.5
			height *= biome_data.height_multiplier
			
			var weight = (biome_data.weight_noise.get_noise_2d(noise_x, noise_z) + 1.0) * 0.5
			weight = smoothstep(biome_data.blend_start, biome_data.blend_end, weight)
			
			section.heights[x].append(height)
			section.weights[x].append(weight)
	
	return section

func get_height(x: int, z: int, section_data: Dictionary) -> float:
	return section_data.heights[x][z]

func get_weight(x: int, z: int, section_data: Dictionary) -> float:
	return section_data.weights[x][z]
