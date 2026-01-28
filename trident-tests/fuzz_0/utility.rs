use crate::types::Offer;
use crate::types::OfferVector;

pub fn find_active_vector_at(offer: &Offer, time: u64) -> Option<OfferVector> {
    offer
        .vectors
        .iter()
        .filter(|vector| vector.start_time != 0 && vector.start_time <= time) // Only consider non-empty vectors
        .max_by_key(|vector| vector.start_time)
        .cloned()
}

pub fn clean_old_vectors(offer: &mut Offer, new_vector: &OfferVector, current_time: u64) {
    // Find currently active vector
    let active_vector = if new_vector.start_time == current_time {
        Some(new_vector.clone())
    } else {
        find_active_vector_at(offer, current_time)
    };

    let active_vector_start_time = match &active_vector {
        Some(vector) => vector.start_time,
        None => return, // No active vector found, nothing to clean
    };

    // Find previously active vector (closest smaller vector_start_timestamp)
    let prev_vector = find_active_vector_at(offer, active_vector.unwrap().start_time - 1);

    let prev_vector_start_time = match prev_vector {
        Some(vector) => vector.start_time,
        None => 0, // If no previous vector exists, use 0
    };

    // Clear all vectors except active and previous
    for vector in offer.vectors.iter_mut() {
        if vector.start_time != 0 // Don't touch already empty slots
            // Keep active vector
            && vector.start_time != active_vector_start_time
            // Keep previous vector
            && vector.start_time != prev_vector_start_time
            // Keep all future vectors
            && vector.start_time < active_vector_start_time
        {
            // emit!(OfferVectorRetiredEvent {
            //     offer_token_in_mint: offer.token_in_mint,
            //     offer_token_out_mint: offer.token_out_mint,
            //     vector_start_time: vector.start_time
            // });
            *vector = OfferVector::default(); // Clear the vector
        }
    }
}
