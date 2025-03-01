extends Node3D

signal noises_randomized

# Adjust to your actual resource paths:
const PATH_CORRAL   = "res://project/terrain/noise/coralNoise.tres"
const PATH_SAND     = "res://project/terrain/noise/sandNoise.tres"
const PATH_ROCK     = "res://project/terrain/noise/rockNoise.tres"
const PATH_KELP     = "res://project/terrain/noise/kelpNoise.tres"
const PATH_LAVAROCK = "res://project/terrain/noise/lavaRockNoise.tres"

const PATH_SECTION  = "res://project/terrain/noise/sectionNoise.tres"
const PATH_BLEND    = "res://project/terrain/noise/blendNoise.tres"


func _ready():
	#randomize_noises() # TODO: seeds should later be randomized in the network node
	randomize_noises()
	
func randomize_noises():
	var rng = RandomNumberGenerator.new()
	var new_seed = rng.randi()

	var corral_noise = load(PATH_CORRAL) as FastNoiseLite
	corral_noise.seed = new_seed
	ResourceSaver.save(corral_noise, PATH_CORRAL)

	var sand_noise = load(PATH_SAND) as FastNoiseLite
	sand_noise.seed = new_seed
	ResourceSaver.save(sand_noise, PATH_SAND)

	var rock_noise = load(PATH_ROCK) as FastNoiseLite
	rock_noise.seed = new_seed
	ResourceSaver.save(rock_noise, PATH_ROCK)

	var kelp_noise = load(PATH_KELP) as FastNoiseLite
	kelp_noise.seed = new_seed
	ResourceSaver.save(kelp_noise, PATH_KELP)

	var lavarock_noise = load(PATH_LAVAROCK) as FastNoiseLite
	lavarock_noise.seed = new_seed
	ResourceSaver.save(lavarock_noise, PATH_LAVAROCK)

	var section_noise = load(PATH_SECTION) as FastNoiseLite
	section_noise.seed = new_seed
	ResourceSaver.save(section_noise, PATH_SECTION)

	var blend_noise = load(PATH_BLEND) as FastNoiseLite
	blend_noise.seed = new_seed
	ResourceSaver.save(blend_noise, PATH_BLEND)
	
	print("Randomized noise seeds and saved to .tres files")
	emit_signal("noises_randomized")
